//! The client's address, believed only from trusted proxies (S3, SEC-02).
//!
//! `X-Forwarded-For` counts only when the TCP peer is a configured trusted
//! proxy (the Fenrir host gateway on loopback). The chain is walked from
//! the right, skipping trusted hops, and the first untrusted address is the
//! client. A request straight from any other peer is keyed by that peer,
//! whatever headers it sends. (Moved unchanged from auth-service
//! `transport/http/client_ip.rs`.)
//!
//! Only `X-Forwarded-For` is read. `Forwarded`, `X-Real-IP`,
//! `CF-Connecting-IP` and the like are never believed here: deciding which
//! edge header names the real client is the host's job (W9-SPEC §2). The
//! host's static edge takes its configured header only from its own
//! trusted proxies, drops every client-supplied address header and sends
//! `X-Forwarded-For: <client>`; every loopback hop behind it (host gateway,
//! athenos-api, the core) appends its peer. Walking from the right past the
//! trusted loopback hops therefore lands on the client the edge resolved,
//! and anything a client managed to put further left is never reached.
//!
//! [`ClientIpLayer`] resolves it once per request and stores [`ClientIp`]
//! as a request extension; [`ClientIp`] is also an axum extractor (features
//! `axum` for 0.8, `axum07` for 0.7). The peer comes from axum's
//! `ConnectInfo<SocketAddr>` (serve with `into_make_service_with_connect_
//! info::<SocketAddr>()`) or from a [`PeerAddr`] extension.

use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::task::{Context, Poll};

use http::{Extensions, HeaderMap, Request};
use tower::{Layer, Service};

pub const FORWARDED_FOR_HEADER: &str = "x-forwarded-for";

/// Peers whose `X-Forwarded-For` is believed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedProxies(Vec<IpAddr>);

impl TrustedProxies {
    /// `127.0.0.1` and `::1`: the host gateway runs on the same machine.
    pub fn loopback() -> Self {
        Self(vec![
            IpAddr::from([127, 0, 0, 1]),
            IpAddr::from(Ipv6Addr::LOCALHOST),
        ])
    }

    /// Nobody may speak for others: the peer is always the client.
    pub fn none() -> Self {
        Self(Vec::new())
    }

    pub fn new(proxies: impl IntoIterator<Item = IpAddr>) -> Self {
        Self(proxies.into_iter().map(|ip| ip.to_canonical()).collect())
    }

    /// A configured list: comma-separated IPs, `none`, or unset/blank for
    /// [`TrustedProxies::loopback`]. Entries that are not IPs are skipped.
    pub fn parse(raw: Option<&str>) -> Self {
        let Some(raw) = raw.map(str::trim).filter(|raw| !raw.is_empty()) else {
            return Self::loopback();
        };
        if raw.eq_ignore_ascii_case("none") {
            return Self::none();
        }
        Self::new(
            raw.split(',')
                .filter_map(|entry| entry.trim().parse::<IpAddr>().ok()),
        )
    }

    pub fn as_slice(&self) -> &[IpAddr] {
        &self.0
    }
}

impl Default for TrustedProxies {
    fn default() -> Self {
        Self::loopback()
    }
}

/// Resolves the client address of a request.
///
/// * `peer` is the TCP peer (`None` when the transport did not record it).
/// * `trusted_proxies` are the peers allowed to speak for others.
///
/// Returns `None` only when the peer is unknown.
pub fn resolve_client_ip(
    peer: Option<IpAddr>,
    headers: &HeaderMap,
    trusted_proxies: &[IpAddr],
) -> Option<IpAddr> {
    let peer = peer?.to_canonical();
    let is_trusted = |ip: &IpAddr| trusted_proxies.contains(ip);
    if !is_trusted(&peer) {
        return Some(peer);
    }

    // All X-Forwarded-For lines form one list, in order.
    let mut hops: Vec<&str> = Vec::new();
    for value in headers.get_all(FORWARDED_FOR_HEADER) {
        let Ok(text) = value.to_str() else {
            return Some(peer);
        };
        hops.extend(text.split(',').map(str::trim));
    }

    let mut client = peer;
    for hop in hops.iter().rev() {
        let Some(ip) = parse_hop(hop) else {
            // Garbage where a trusted proxy should have written an address:
            // trust nothing to the left of it.
            break;
        };
        client = ip;
        if !is_trusted(&ip) {
            break;
        }
    }
    Some(client)
}

/// One `X-Forwarded-For` element: a bare IP, `ip:port`, `[v6]` or
/// `[v6]:port`. Anything else (names, zone ids, `unknown`, obfuscated
/// identifiers) is garbage.
fn parse_hop(raw: &str) -> Option<IpAddr> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let bracketed = raw
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'));
    raw.parse::<IpAddr>()
        .ok()
        .or_else(|| raw.parse::<SocketAddr>().ok().map(|addr| addr.ip()))
        .or_else(|| bracketed.and_then(|inner| inner.parse::<Ipv6Addr>().ok().map(IpAddr::V6)))
        .map(|ip| ip.to_canonical())
}

/// The TCP peer, for servers that do not record axum's `ConnectInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerAddr(pub SocketAddr);

/// The TCP peer of a request, from whatever the server recorded.
pub fn peer_of(extensions: &Extensions) -> Option<IpAddr> {
    if let Some(PeerAddr(addr)) = extensions.get::<PeerAddr>() {
        return Some(addr.ip());
    }
    #[cfg(feature = "axum")]
    if let Some(info) = extensions.get::<axum::extract::ConnectInfo<SocketAddr>>() {
        return Some(info.0.ip());
    }
    #[cfg(feature = "axum07")]
    if let Some(info) = extensions.get::<axum07::extract::ConnectInfo<SocketAddr>>() {
        return Some(info.0.ip());
    }
    None
}

/// The resolved client address, stored as a request extension by
/// [`ClientIpLayer`] and readable as an extractor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClientIp(pub Option<IpAddr>);

impl ClientIp {
    /// What [`ClientIpLayer`] stored, or unknown.
    pub fn from_extensions(extensions: &Extensions) -> Self {
        extensions.get::<ClientIp>().copied().unwrap_or_default()
    }

    /// The address as text, or `"unknown"`.
    pub fn key(&self) -> String {
        self.0
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub fn as_string(&self) -> Option<String> {
        self.0.map(|ip| ip.to_string())
    }
}

#[cfg(feature = "axum")]
impl<S: Send + Sync> axum::extract::FromRequestParts<S> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self::from_extensions(&parts.extensions))
    }
}

#[cfg(feature = "axum07")]
#[async_trait::async_trait]
impl<S: Send + Sync> axum07::extract::FromRequestParts<S> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self::from_extensions(&parts.extensions))
    }
}

/// Resolves [`ClientIp`] once per request.
#[derive(Debug, Clone)]
pub struct ClientIpLayer {
    trusted: Arc<TrustedProxies>,
}

impl ClientIpLayer {
    pub fn new(trusted: TrustedProxies) -> Self {
        Self {
            trusted: Arc::new(trusted),
        }
    }
}

impl<S> Layer<S> for ClientIpLayer {
    type Service = ClientIpService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ClientIpService {
            inner,
            trusted: Arc::clone(&self.trusted),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientIpService<S> {
    inner: S,
    trusted: Arc<TrustedProxies>,
}

impl<S, B> Service<Request<B>> for ClientIpService<S>
where
    S: Service<Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request<B>) -> Self::Future {
        let peer = peer_of(request.extensions());
        let client = resolve_client_ip(peer, request.headers(), self.trusted.as_slice());
        request.extensions_mut().insert(ClientIp(client));
        self.inner.call(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;
    use tower::ServiceExt;

    fn loopback() -> Vec<IpAddr> {
        TrustedProxies::loopback().as_slice().to_vec()
    }

    fn ip(raw: &str) -> IpAddr {
        raw.parse().unwrap()
    }

    fn xff(values: &[&str]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for value in values {
            headers.append(FORWARDED_FOR_HEADER, HeaderValue::from_str(value).unwrap());
        }
        headers
    }

    #[test]
    fn an_untrusted_peer_cannot_choose_its_address() {
        let headers = xff(&["10.0.0.7"]);
        assert_eq!(
            resolve_client_ip(Some(ip("192.168.1.20")), &headers, &loopback()),
            Some(ip("192.168.1.20"))
        );
    }

    #[test]
    fn a_trusted_proxy_forwards_the_client() {
        let headers = xff(&["203.0.113.9"]);
        assert_eq!(
            resolve_client_ip(Some(ip("127.0.0.1")), &headers, &loopback()),
            Some(ip("203.0.113.9"))
        );
    }

    #[test]
    fn the_rightmost_untrusted_hop_wins() {
        let headers = xff(&["1.2.3.4, 203.0.113.9, 127.0.0.1"]);
        assert_eq!(
            resolve_client_ip(Some(ip("127.0.0.1")), &headers, &loopback()),
            Some(ip("203.0.113.9"))
        );
        let split = xff(&["1.2.3.4", "203.0.113.9"]);
        assert_eq!(
            resolve_client_ip(Some(ip("127.0.0.1")), &split, &loopback()),
            Some(ip("203.0.113.9"))
        );
    }

    #[test]
    fn a_trusted_peer_without_header_is_the_client() {
        assert_eq!(
            resolve_client_ip(Some(ip("127.0.0.1")), &HeaderMap::new(), &loopback()),
            Some(ip("127.0.0.1"))
        );
    }

    #[test]
    fn garbage_stops_the_walk() {
        let headers = xff(&["1.2.3.4, not-an-ip"]);
        assert_eq!(
            resolve_client_ip(Some(ip("127.0.0.1")), &headers, &loopback()),
            Some(ip("127.0.0.1"))
        );
    }

    #[test]
    fn ports_and_mapped_addresses_are_normalised() {
        let headers = xff(&["203.0.113.9:5555"]);
        assert_eq!(
            resolve_client_ip(Some(ip("::ffff:127.0.0.1")), &headers, &loopback()),
            Some(ip("203.0.113.9"))
        );
        let v6 = xff(&["[2001:db8::1]:443"]);
        assert_eq!(
            resolve_client_ip(Some(ip("::1")), &v6, &loopback()),
            Some(ip("2001:db8::1"))
        );
    }

    /// The W9 chain: the host edge resolved the client and set
    /// `X-Forwarded-For: <client>`; the gateway, athenos-api and the core
    /// each appended their loopback peer on the way to this service.
    #[test]
    fn the_host_chain_resolves_to_the_edge_client() {
        let headers = xff(&["198.51.100.23, 127.0.0.1, 127.0.0.1, ::1"]);
        assert_eq!(
            resolve_client_ip(Some(ip("127.0.0.1")), &headers, &loopback()),
            Some(ip("198.51.100.23"))
        );
        let v6 = xff(&["2001:db8:7::42", "127.0.0.1"]);
        assert_eq!(
            resolve_client_ip(Some(ip("::1")), &v6, &loopback()),
            Some(ip("2001:db8:7::42"))
        );
    }

    /// Whatever a client managed to put to the left of the address the edge
    /// resolved is never reached: the walk stops at the first untrusted hop.
    #[test]
    fn a_client_supplied_chain_cannot_replace_the_resolved_client() {
        for spoof in [
            "127.0.0.1, 198.51.100.23, 127.0.0.1",
            "10.0.0.1, 1.1.1.1, 198.51.100.23, 127.0.0.1",
            "::1, 198.51.100.23, ::1",
        ] {
            assert_eq!(
                resolve_client_ip(Some(ip("127.0.0.1")), &xff(&[spoof]), &loopback()),
                Some(ip("198.51.100.23")),
                "{spoof}"
            );
        }
        // Split over several header lines: still one list, in order.
        let lines = xff(&["6.6.6.6", "198.51.100.23", "127.0.0.1"]);
        assert_eq!(
            resolve_client_ip(Some(ip("127.0.0.1")), &lines, &loopback()),
            Some(ip("198.51.100.23"))
        );
    }

    /// Other address headers are never believed at module level, from any
    /// peer: only the host decides which edge header names the client.
    #[test]
    fn other_address_headers_are_ignored() {
        let mut headers = xff(&["198.51.100.23"]);
        for (name, value) in [
            ("forwarded", "for=6.6.6.6;proto=https"),
            ("x-real-ip", "6.6.6.6"),
            ("cf-connecting-ip", "6.6.6.6"),
            ("true-client-ip", "6.6.6.6"),
            ("x-client-ip", "6.6.6.6"),
        ] {
            headers.insert(name, HeaderValue::from_static(value));
        }
        assert_eq!(
            resolve_client_ip(Some(ip("127.0.0.1")), &headers, &loopback()),
            Some(ip("198.51.100.23"))
        );
        let mut only_others = HeaderMap::new();
        only_others.insert("x-real-ip", HeaderValue::from_static("6.6.6.6"));
        only_others.insert("cf-connecting-ip", HeaderValue::from_static("6.6.6.6"));
        only_others.insert("forwarded", HeaderValue::from_static("for=6.6.6.6"));
        assert_eq!(
            resolve_client_ip(Some(ip("127.0.0.1")), &only_others, &loopback()),
            Some(ip("127.0.0.1"))
        );
        // An untrusted peer gets nothing out of any of them.
        assert_eq!(
            resolve_client_ip(Some(ip("203.0.113.5")), &headers, &loopback()),
            Some(ip("203.0.113.5"))
        );
    }

    #[test]
    fn ipv6_forms_parse_and_garbage_does_not() {
        for (hop, want) in [
            ("2001:db8::1", "2001:db8::1"),
            ("[2001:db8::1]", "2001:db8::1"),
            ("[2001:db8::1]:8443", "2001:db8::1"),
            ("::ffff:198.51.100.23", "198.51.100.23"),
            ("198.51.100.23:61000", "198.51.100.23"),
        ] {
            assert_eq!(parse_hop(hop), Some(ip(want)), "{hop}");
        }
        for garbage in [
            "unknown",
            "_hidden",
            "example.com",
            "fe80::1%eth0",
            "[2001:db8::1",
            "2001:db8::1]",
            "1.2.3",
            "1.2.3.4.5",
            "999.1.1.1",
            "for=1.2.3.4",
            "\"1.2.3.4\"",
        ] {
            assert_eq!(parse_hop(garbage), None, "{garbage}");
        }
        // Garbage right of the client hides it: the last trusted hop stays.
        let headers = xff(&["198.51.100.23, unknown, 127.0.0.1"]);
        assert_eq!(
            resolve_client_ip(Some(ip("127.0.0.1")), &headers, &loopback()),
            Some(ip("127.0.0.1"))
        );
        // Header bytes that are not text: the peer.
        let mut binary = HeaderMap::new();
        binary.insert(
            FORWARDED_FOR_HEADER,
            HeaderValue::from_bytes(b"198.51.100.23\xff").unwrap(),
        );
        assert_eq!(
            resolve_client_ip(Some(ip("127.0.0.1")), &binary, &loopback()),
            Some(ip("127.0.0.1"))
        );
    }

    #[test]
    fn no_trusted_proxies_means_the_peer_always() {
        let headers = xff(&["203.0.113.9"]);
        assert_eq!(
            resolve_client_ip(Some(ip("127.0.0.1")), &headers, &[]),
            Some(ip("127.0.0.1"))
        );
    }

    #[test]
    fn an_unknown_peer_resolves_to_nothing() {
        assert_eq!(
            resolve_client_ip(None, &xff(&["203.0.113.9"]), &loopback()),
            None
        );
    }

    #[test]
    fn the_configured_list_parses() {
        assert_eq!(TrustedProxies::parse(None), TrustedProxies::loopback());
        assert_eq!(TrustedProxies::parse(Some(" ")), TrustedProxies::loopback());
        assert!(TrustedProxies::parse(Some("none")).as_slice().is_empty());
        assert_eq!(
            TrustedProxies::parse(Some("10.0.0.1, bogus, ::ffff:10.0.0.2")).as_slice(),
            &[ip("10.0.0.1"), ip("10.0.0.2")]
        );
    }

    #[tokio::test]
    async fn the_layer_stores_the_client_for_the_handler() {
        let service = ClientIpLayer::new(TrustedProxies::loopback()).layer(tower::service_fn(
            |request: Request<()>| async move {
                Ok::<_, std::convert::Infallible>(ClientIp::from_extensions(request.extensions()))
            },
        ));
        let mut request = Request::new(());
        request
            .extensions_mut()
            .insert(PeerAddr("127.0.0.1:5000".parse().unwrap()));
        request.headers_mut().insert(
            FORWARDED_FOR_HEADER,
            HeaderValue::from_static("198.51.100.7"),
        );
        let client = service.oneshot(request).await.unwrap();
        assert_eq!(client, ClientIp(Some(ip("198.51.100.7"))));
        assert_eq!(client.key(), "198.51.100.7");
        assert_eq!(ClientIp(None).key(), "unknown");
    }
}
