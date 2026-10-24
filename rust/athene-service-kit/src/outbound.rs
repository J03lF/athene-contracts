//! The client for user-supplied URLs (webhooks, link previews, imports by
//! URL), guarded against SSRF (SEC-25, Q-043). Not wired anywhere yet: the
//! first service that sends to a URL a user chose goes through this, and
//! only this.
//!
//! - **Scheme and port allowlists** (default `https` on 443); no user info
//!   in the URL.
//! - **Addresses are checked after DNS resolution**, by the resolver the
//!   connection itself uses: every address a name resolves to must be
//!   public, or the call fails — one private address in the answer is
//!   enough. Because the check happens where the socket is opened, a name
//!   that resolves to a public address for a pre-check and to `127.0.0.1`
//!   a moment later (DNS rebinding) is still refused. IP literals —
//!   including `2130706433`, `0x7f.1`, `[::ffff:127.0.0.1]` — are checked
//!   the same way.
//! - Blocked: loopback, private (RFC 1918, ULA), link-local (incl. the
//!   cloud metadata address `169.254.169.254`), CGNAT, multicast,
//!   documentation, benchmarking and reserved ranges; IPv4-mapped,
//!   -compatible and NAT64 addresses by the IPv4 address they embed; 6to4
//!   and Teredo outright.
//! - No proxy, no redirects (a redirect is answered, never followed), a
//!   connect timeout, a total timeout and a size cap on both bodies.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode};
use url::{Host, Url};

use crate::util::BoxFuture;

/// What a call may reach.
#[derive(Debug, Clone)]
pub struct OutboundPolicy {
    /// Lower-case schemes; default `https`.
    pub schemes: Vec<String>,
    /// Ports (explicit or the scheme's default); default 443.
    pub ports: Vec<u16>,
    /// Addresses allowed although they are not public (an operator's
    /// internal sink, a test server). Empty by default.
    pub permitted: Vec<IpAddr>,
    pub max_request_bytes: usize,
    pub max_response_bytes: usize,
    pub connect_timeout: Duration,
    /// The whole call.
    pub timeout: Duration,
}

impl Default for OutboundPolicy {
    fn default() -> Self {
        Self {
            schemes: vec!["https".to_string()],
            ports: vec![443],
            permitted: Vec::new(),
            max_request_bytes: 1024 * 1024,
            max_response_bytes: 1024 * 1024,
            connect_timeout: Duration::from_secs(3),
            timeout: Duration::from_secs(10),
        }
    }
}

impl OutboundPolicy {
    fn is_permitted(&self, ip: IpAddr) -> bool {
        self.permitted.contains(&ip.to_canonical())
    }

    /// Why `ip` may not be reached under this policy, if it may not.
    pub fn refusal(&self, ip: IpAddr) -> Option<&'static str> {
        if self.is_permitted(ip) {
            return None;
        }
        blocked_reason(ip)
    }
}

/// Why `ip` is not a public unicast address, if it is not.
pub fn blocked_reason(ip: IpAddr) -> Option<&'static str> {
    // IPv4-mapped (`::ffff:a.b.c.d`) counts as the IPv4 address.
    match ip.to_canonical() {
        IpAddr::V4(v4) => blocked_v4(v4),
        IpAddr::V6(v6) => blocked_v6(v6),
    }
}

fn blocked_v4(ip: Ipv4Addr) -> Option<&'static str> {
    let [a, b, c, _] = ip.octets();
    let reason = match (a, b, c) {
        (0, _, _) => "this-network",
        (10, _, _) => "private",
        (100, 64..=127, _) => "shared-address-space",
        (127, _, _) => "loopback",
        (169, 254, _) => "link-local",
        (172, 16..=31, _) => "private",
        (192, 0, 0) => "ietf-protocol-assignment",
        (192, 0, 2) | (198, 51, 100) | (203, 0, 113) => "documentation",
        (192, 88, 99) => "6to4-relay",
        (192, 168, _) => "private",
        (198, 18..=19, _) => "benchmarking",
        (224..=239, _, _) => "multicast",
        (240..=255, _, _) => "reserved",
        _ => return None,
    };
    Some(reason)
}

fn blocked_v6(ip: Ipv6Addr) -> Option<&'static str> {
    let segments = ip.segments();
    if ip.is_unspecified() {
        return Some("unspecified");
    }
    if ip.is_loopback() {
        return Some("loopback");
    }
    // ::/96, IPv4-compatible (deprecated).
    if segments[..6] == [0, 0, 0, 0, 0, 0] {
        return Some("ipv4-compatible");
    }
    // 64:ff9b::/96 NAT64: as the IPv4 address it reaches.
    if segments[..6] == [0x64, 0xff9b, 0, 0, 0, 0] {
        let [a, b] = segments[6].to_be_bytes();
        let [c, d] = segments[7].to_be_bytes();
        let embedded = Ipv4Addr::new(a, b, c, d);
        return blocked_v4(embedded).map(|_| "nat64-non-public");
    }
    let reason = match segments[0] {
        // 64:ff9b:1::/48 local-use NAT64.
        0x0064 if segments[1] == 0xff9b => "nat64-local",
        // 100::/64 discard-only.
        0x0100 if segments[1..4] == [0, 0, 0] => "discard",
        // 2001::/23 IETF protocol assignments (Teredo 2001::/32 among them).
        0x2001 if segments[1] < 0x0200 => "ietf-protocol-assignment",
        0x2001 if segments[1] == 0x0db8 => "documentation",
        0x2002 => "6to4",
        0x3fff if segments[1] < 0x1000 => "documentation",
        first if first & 0xfe00 == 0xfc00 => "unique-local",
        first if first & 0xffc0 == 0xfe80 => "link-local",
        first if first & 0xffc0 == 0xfec0 => "site-local",
        first if first & 0xff00 == 0xff00 => "multicast",
        // Outside 2000::/3 nothing is global unicast today.
        first if first & 0xe000 != 0x2000 => "reserved",
        _ => return None,
    };
    Some(reason)
}

/// Why an outbound call was refused or failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OutboundError {
    #[error("not a valid URL: {0}")]
    Url(String),
    #[error("scheme '{0}' is not allowed")]
    Scheme(String),
    #[error("port {0} is not allowed")]
    Port(u16),
    #[error("credentials in the URL are not allowed")]
    UserInfo,
    #[error("the URL has no host")]
    NoHost,
    #[error("{ip} is not a public address ({reason})")]
    Blocked { ip: IpAddr, reason: &'static str },
    #[error("the host name did not resolve: {0}")]
    Resolve(String),
    #[error("the request body is larger than {0} bytes")]
    RequestTooLarge(usize),
    #[error("the response is larger than {0} bytes")]
    ResponseTooLarge(usize),
    #[error("the call timed out")]
    Timeout,
    #[error("transport failure: {0}")]
    Transport(String),
    #[error("invalid client configuration: {0}")]
    Config(String),
}

/// Resolves host names (the system resolver in production; a fixed table
/// in tests).
pub trait HostResolver: Send + Sync {
    fn resolve<'a>(
        &'a self,
        host: &'a str,
        port: u16,
    ) -> BoxFuture<'a, std::io::Result<Vec<SocketAddr>>>;
}

/// The operating system's resolver.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemResolver;

impl HostResolver for SystemResolver {
    fn resolve<'a>(
        &'a self,
        host: &'a str,
        port: u16,
    ) -> BoxFuture<'a, std::io::Result<Vec<SocketAddr>>> {
        Box::pin(async move { Ok(tokio::net::lookup_host((host, port)).await?.collect()) })
    }
}

/// Every address `addrs` holds must pass `policy`.
fn check_all(policy: &OutboundPolicy, addrs: &[SocketAddr]) -> Result<(), OutboundError> {
    if addrs.is_empty() {
        return Err(OutboundError::Resolve("no addresses".to_string()));
    }
    for addr in addrs {
        if let Some(reason) = policy.refusal(addr.ip()) {
            return Err(OutboundError::Blocked {
                ip: addr.ip().to_canonical(),
                reason,
            });
        }
    }
    Ok(())
}

/// The refusal the connection-time resolver reports to reqwest.
#[derive(Debug)]
struct Refused(OutboundError);

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for Refused {}

fn refused(err: OutboundError) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(Refused(err))
}

/// The resolver reqwest connects through: it checks every answer.
struct GuardedResolver {
    resolver: Arc<dyn HostResolver>,
    policy: Arc<OutboundPolicy>,
}

impl reqwest::dns::Resolve for GuardedResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let resolver = Arc::clone(&self.resolver);
        let policy = Arc::clone(&self.policy);
        let host = name.as_str().to_string();
        Box::pin(async move {
            // The port is replaced by the URL's own when connecting.
            let addrs = resolver
                .resolve(&host, 0)
                .await
                .map_err(|err| refused(OutboundError::Resolve(err.to_string())))?;
            check_all(&policy, &addrs).map_err(refused)?;
            let addrs: reqwest::dns::Addrs = Box::new(addrs.into_iter());
            Ok(addrs)
        })
    }
}

/// An answer, read up to the size cap.
#[derive(Debug, Clone)]
pub struct OutboundResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

pub struct OutboundClient {
    policy: Arc<OutboundPolicy>,
    resolver: Arc<dyn HostResolver>,
    http: reqwest::Client,
}

impl fmt::Debug for OutboundClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutboundClient")
            .field("policy", &self.policy)
            .finish_non_exhaustive()
    }
}

impl OutboundClient {
    pub fn new(policy: OutboundPolicy) -> Result<Self, OutboundError> {
        Self::with_resolver(policy, Arc::new(SystemResolver))
    }

    pub fn with_resolver(
        policy: OutboundPolicy,
        resolver: Arc<dyn HostResolver>,
    ) -> Result<Self, OutboundError> {
        let policy = Arc::new(policy);
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(policy.connect_timeout)
            .timeout(policy.timeout)
            .dns_resolver(Arc::new(GuardedResolver {
                resolver: Arc::clone(&resolver),
                policy: Arc::clone(&policy),
            }))
            .build()
            .map_err(|err| OutboundError::Config(err.to_string()))?;
        Ok(Self {
            policy,
            resolver,
            http,
        })
    }

    pub fn policy(&self) -> &OutboundPolicy {
        &self.policy
    }

    /// `url`, if this client may call it: scheme, port, no user info, and
    /// every address of its host public. The connection checks the
    /// addresses again ([`OutboundClient::send`] always does both).
    pub async fn check(&self, url: &str) -> Result<Url, OutboundError> {
        let url = Url::parse(url.trim()).map_err(|err| OutboundError::Url(err.to_string()))?;
        if !self
            .policy
            .schemes
            .iter()
            .any(|scheme| scheme == url.scheme())
        {
            return Err(OutboundError::Scheme(url.scheme().to_string()));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(OutboundError::UserInfo);
        }
        let port = url.port_or_known_default().ok_or(OutboundError::NoHost)?;
        if !self.policy.ports.contains(&port) {
            return Err(OutboundError::Port(port));
        }
        let addrs = match url.host().ok_or(OutboundError::NoHost)? {
            Host::Ipv4(ip) => vec![SocketAddr::new(IpAddr::V4(ip), port)],
            Host::Ipv6(ip) => vec![SocketAddr::new(IpAddr::V6(ip), port)],
            Host::Domain(name) => self
                .resolver
                .resolve(name, port)
                .await
                .map_err(|err| OutboundError::Resolve(err.to_string()))?,
        };
        check_all(&self.policy, &addrs)?;
        Ok(url)
    }

    /// One call: checked, sent without following redirects, and read up to
    /// the size cap.
    pub async fn send(
        &self,
        method: Method,
        url: &str,
        headers: HeaderMap,
        body: Option<Bytes>,
    ) -> Result<OutboundResponse, OutboundError> {
        let url = self.check(url).await?;
        let mut request = self.http.request(method, url).headers(headers);
        if let Some(body) = body {
            if body.len() > self.policy.max_request_bytes {
                return Err(OutboundError::RequestTooLarge(
                    self.policy.max_request_bytes,
                ));
            }
            request = request.body(body);
        }
        let mut response = request.send().await.map_err(send_error)?;
        let cap = self.policy.max_response_bytes;
        if response
            .content_length()
            .is_some_and(|length| usize::try_from(length).map_or(true, |length| length > cap))
        {
            return Err(OutboundError::ResponseTooLarge(cap));
        }
        let status = response.status();
        let headers = response.headers().clone();
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(send_error)? {
            if body.len() + chunk.len() > cap {
                return Err(OutboundError::ResponseTooLarge(cap));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(OutboundResponse {
            status,
            headers,
            body: Bytes::from(body),
        })
    }

    /// `POST` with a JSON body.
    pub async fn post_json<T: serde::Serialize>(
        &self,
        url: &str,
        body: &T,
    ) -> Result<OutboundResponse, OutboundError> {
        let json =
            serde_json::to_vec(body).map_err(|err| OutboundError::Config(err.to_string()))?;
        let mut headers = HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("application/json"),
        );
        self.send(Method::POST, url, headers, Some(Bytes::from(json)))
            .await
    }
}

/// A send failure: the guard's own refusal when it was one.
fn send_error(err: reqwest::Error) -> OutboundError {
    let mut source = std::error::Error::source(&err);
    while let Some(cause) = source {
        if let Some(Refused(refusal)) = cause.downcast_ref::<Refused>() {
            return refusal.clone();
        }
        source = cause.source();
    }
    if err.is_timeout() {
        OutboundError::Timeout
    } else {
        OutboundError::Transport(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use axum::routing::get;
    use axum::Router;

    use super::*;

    fn ip(raw: &str) -> IpAddr {
        raw.parse().unwrap()
    }

    #[test]
    fn non_public_ipv4_ranges_are_blocked() {
        for raw in [
            "0.0.0.0",
            "10.0.0.1",
            "100.64.0.1",
            "127.0.0.1",
            "127.255.255.254",
            "169.254.169.254",
            "172.16.0.1",
            "172.31.255.255",
            "192.0.0.8",
            "192.0.2.1",
            "192.168.1.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "240.0.0.1",
            "255.255.255.255",
        ] {
            assert!(blocked_reason(ip(raw)).is_some(), "{raw}");
        }
        for raw in [
            "1.1.1.1",
            "8.8.8.8",
            "93.184.216.34",
            "172.32.0.1",
            "100.128.0.1",
        ] {
            assert_eq!(blocked_reason(ip(raw)), None, "{raw}");
        }
    }

    #[test]
    fn non_public_ipv6_ranges_are_blocked() {
        for raw in [
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.1",
            "::ffff:169.254.169.254",
            "::127.0.0.1",
            "64:ff9b::a00:1",
            "64:ff9b::7f00:1",
            "64:ff9b:1::1",
            "100::1",
            "2001::1",
            "2001:db8::1",
            "2002:c0a8:101::1",
            "3fff::1",
            "fc00::1",
            "fd12:3456::1",
            "fe80::1",
            "fec0::1",
            "ff02::1",
            "4000::1",
        ] {
            assert!(blocked_reason(ip(raw)).is_some(), "{raw}");
        }
        for raw in [
            "2606:4700:4700::1111",
            "2a00:1450:4001:80b::200e",
            "::ffff:8.8.8.8",
            "64:ff9b::808:808",
        ] {
            assert_eq!(blocked_reason(ip(raw)), None, "{raw}");
        }
    }

    /// Answers from a fixed table; with `rebind`, only the first lookup
    /// gets the table's answer and every later one `127.0.0.1`.
    #[derive(Default)]
    struct Table {
        entries: HashMap<String, Vec<IpAddr>>,
        rebind: bool,
        lookups: Arc<AtomicUsize>,
    }

    impl Table {
        fn with(entries: &[(&str, &[&str])]) -> Self {
            Self {
                entries: entries
                    .iter()
                    .map(|(name, addrs)| (name.to_string(), addrs.iter().map(|a| ip(a)).collect()))
                    .collect(),
                ..Self::default()
            }
        }
    }

    impl HostResolver for Table {
        fn resolve<'a>(
            &'a self,
            host: &'a str,
            port: u16,
        ) -> BoxFuture<'a, std::io::Result<Vec<SocketAddr>>> {
            let lookup = self.lookups.fetch_add(1, Ordering::SeqCst);
            let answer = if self.rebind && lookup > 0 {
                Some(vec![ip("127.0.0.1")])
            } else {
                self.entries.get(host).cloned()
            };
            Box::pin(async move {
                answer
                    .map(|addrs| {
                        addrs
                            .into_iter()
                            .map(|ip| SocketAddr::new(ip, port))
                            .collect()
                    })
                    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "nxdomain"))
            })
        }
    }

    fn client(policy: OutboundPolicy, table: Table) -> OutboundClient {
        OutboundClient::with_resolver(policy, Arc::new(table)).unwrap()
    }

    #[tokio::test]
    async fn urls_are_checked_before_anything_is_sent() {
        let table = Table::with(&[
            ("hooks.example.com", &["93.184.216.34"]),
            ("internal.example.com", &["10.0.0.5"]),
            ("mixed.example.com", &["93.184.216.34", "127.0.0.1"]),
            ("v6-ula.example.com", &["fd00::1"]),
            ("metadata.example.com", &["169.254.169.254"]),
        ]);
        let client = client(OutboundPolicy::default(), table);
        assert!(client.check("https://hooks.example.com/x").await.is_ok());
        assert!(client.check("https://93.184.216.34/x").await.is_ok());

        let blocked = |result: Result<Url, OutboundError>| {
            matches!(result, Err(OutboundError::Blocked { .. }))
        };
        for url in [
            "https://internal.example.com/",
            "https://mixed.example.com/",
            "https://v6-ula.example.com/",
            "https://metadata.example.com/latest/meta-data/",
            "https://127.0.0.1/",
            "https://[::1]/",
            "https://[::ffff:127.0.0.1]/",
            "https://[fe80::1]/",
            // Integer, hex and octal spellings of 127.0.0.1.
            "https://2130706433/",
            "https://0x7f.1/",
            "https://0177.0.0.1/",
            "https://10.1/",
        ] {
            assert!(blocked(client.check(url).await), "{url}");
        }
        assert_eq!(
            client.check("http://hooks.example.com/").await,
            Err(OutboundError::Scheme("http".to_string()))
        );
        assert_eq!(
            client.check("file:///etc/passwd").await,
            Err(OutboundError::Scheme("file".to_string()))
        );
        assert_eq!(
            client.check("gopher://hooks.example.com/").await,
            Err(OutboundError::Scheme("gopher".to_string()))
        );
        assert_eq!(
            client.check("https://hooks.example.com:22/").await,
            Err(OutboundError::Port(22))
        );
        assert_eq!(
            client.check("https://user:pw@hooks.example.com/").await,
            Err(OutboundError::UserInfo)
        );
        // The part after `@` is the host.
        assert_eq!(
            client.check("https://hooks.example.com@127.0.0.1/").await,
            Err(OutboundError::UserInfo)
        );
        assert!(matches!(
            client.check("https://nxdomain.example.com/").await,
            Err(OutboundError::Resolve(_))
        ));
    }

    #[derive(Clone, Default)]
    struct Hits(Arc<Mutex<HashMap<&'static str, usize>>>);

    impl Hits {
        fn hit(&self, path: &'static str) {
            *self.0.lock().unwrap().entry(path).or_default() += 1;
        }
        fn of(&self, path: &str) -> usize {
            self.0.lock().unwrap().get(path).copied().unwrap_or(0)
        }
    }

    async fn sink(hits: Hits) -> u16 {
        let counted = |path: &'static str, body: &'static str| {
            let hits = hits.clone();
            get(move || {
                let hits = hits.clone();
                async move {
                    hits.hit(path);
                    body
                }
            })
        };
        let app = Router::new()
            .route("/hook", counted("/hook", "ok"))
            .route("/secret", counted("/secret", "secret"))
            .route("/big", get(|| async { "x".repeat(4096) }))
            .route(
                "/redirect",
                get(|| async {
                    (
                        StatusCode::FOUND,
                        [(http::header::LOCATION, "/secret")],
                        "moved",
                    )
                }),
            )
            .route(
                "/slow",
                get(|| async {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    "late"
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        port
    }

    /// A policy for the local test server: plain http on its port, and
    /// 127.0.0.1 explicitly permitted.
    fn local_policy(port: u16) -> OutboundPolicy {
        OutboundPolicy {
            schemes: vec!["http".to_string()],
            ports: vec![port],
            permitted: vec![ip("127.0.0.1")],
            max_response_bytes: 1024,
            timeout: Duration::from_millis(500),
            ..OutboundPolicy::default()
        }
    }

    #[tokio::test]
    async fn a_permitted_sink_is_called_without_following_redirects_or_reading_too_much() {
        let hits = Hits::default();
        let port = sink(hits.clone()).await;
        let client = client(
            local_policy(port),
            Table::with(&[("hooks.test", &["127.0.0.1"])]),
        );
        let base = format!("http://hooks.test:{port}");

        let response = client
            .send(Method::GET, &format!("{base}/hook"), HeaderMap::new(), None)
            .await
            .unwrap();
        assert_eq!(
            (response.status, &response.body[..]),
            (StatusCode::OK, &b"ok"[..])
        );

        let redirect = client
            .send(
                Method::GET,
                &format!("{base}/redirect"),
                HeaderMap::new(),
                None,
            )
            .await
            .unwrap();
        assert_eq!(redirect.status, StatusCode::FOUND);
        assert_eq!(hits.of("/secret"), 0, "the redirect is not followed");

        assert_eq!(
            client
                .send(Method::GET, &format!("{base}/big"), HeaderMap::new(), None)
                .await
                .unwrap_err(),
            OutboundError::ResponseTooLarge(1024)
        );
        assert_eq!(
            client
                .send(
                    Method::POST,
                    &format!("{base}/hook"),
                    HeaderMap::new(),
                    Some(Bytes::from(vec![b'x'; 2 * 1024 * 1024]))
                )
                .await
                .unwrap_err(),
            OutboundError::RequestTooLarge(1024 * 1024)
        );
        assert_eq!(
            client
                .send(Method::GET, &format!("{base}/slow"), HeaderMap::new(), None)
                .await
                .unwrap_err(),
            OutboundError::Timeout
        );
    }

    #[tokio::test]
    async fn without_the_exception_the_local_sink_is_unreachable() {
        let hits = Hits::default();
        let port = sink(hits.clone()).await;
        let policy = OutboundPolicy {
            permitted: Vec::new(),
            ..local_policy(port)
        };
        let client = client(policy, Table::with(&[("hooks.test", &["127.0.0.1"])]));
        let err = client
            .send(
                Method::GET,
                &format!("http://hooks.test:{port}/hook"),
                HeaderMap::new(),
                None,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                OutboundError::Blocked {
                    reason: "loopback",
                    ..
                }
            ),
            "{err:?}"
        );
        let err = client
            .send(
                Method::GET,
                &format!("http://127.0.0.1:{port}/hook"),
                HeaderMap::new(),
                None,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, OutboundError::Blocked { .. }), "{err:?}");
        assert_eq!(hits.of("/hook"), 0);
    }

    /// DNS rebinding: the name is public when the URL is checked and
    /// loopback when the connection resolves it. The check at connect time
    /// refuses it; the sink is never reached.
    #[tokio::test]
    async fn a_name_that_rebinds_to_loopback_is_refused_at_connect_time() {
        let hits = Hits::default();
        let port = sink(hits.clone()).await;
        let policy = OutboundPolicy {
            permitted: Vec::new(),
            ..local_policy(port)
        };
        let table = Table {
            rebind: true,
            ..Table::with(&[("rebind.test", &["93.184.216.34"])])
        };
        let lookups = Arc::clone(&table.lookups);
        let client = client(policy, table);
        let err = client
            .send(
                Method::GET,
                &format!("http://rebind.test:{port}/hook"),
                HeaderMap::new(),
                None,
            )
            .await
            .unwrap_err();
        assert_eq!(
            err,
            OutboundError::Blocked {
                ip: ip("127.0.0.1"),
                reason: "loopback"
            }
        );
        assert_eq!(
            lookups.load(Ordering::SeqCst),
            2,
            "the pre-check saw the public answer; the connection's own lookup was refused"
        );
        assert_eq!(hits.of("/hook"), 0);
    }
}
