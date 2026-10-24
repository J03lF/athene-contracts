//! Rate limits: one bounded limiter, keys by IP, token or account, and a
//! layer per route class (Q-044, SEC-19, SEC-03).
//!
//! [`RateLimiter`] (moved from auth-service `security/rate_limiter.rs`)
//! keeps a sliding window per key. The number of keys is capped: at the cap,
//! stale keys go first and then the least recently used, so a flood of fresh
//! keys (random `X-Forwarded-For`, random tokens) costs a bounded amount of
//! memory instead of growing without limit — the gateway's old map never
//! forgot a key (SEC-19).
//!
//! Keys never hold a credential: a token is counted under a prefix of its
//! SHA-256 ([`token_key`]). An authenticated caller is counted per token, so
//! the users behind one proxy address no longer share one bucket (the
//! `global:127.0.0.1` bucket that held every user, SEC-03).
//!
//! [`RateLimitLayer`] counts every request of one [`RateClass`] — the
//! classes the gateway's route table declares (step 3.2): `Standard` for
//! all traffic, `Credential` and `Stream` on top for the routes that carry
//! them (`route_layer`), each with its own [`RateRule`].

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use http::header::AUTHORIZATION;
use http::{HeaderMap, Request, Response};
use sha2::{Digest, Sha256};
use tower::{Layer, Service};

use crate::client_ip::ClientIp;
use crate::util::{BoxFuture, Render};

/// How often stale keys are swept even when the cap is not reached.
const SWEEP_INTERVAL: Duration = Duration::from_secs(30);

/// Hex digits of a token's SHA-256 kept in its key: enough to separate
/// callers, and the key never holds the token itself.
const TOKEN_KEY_HEX: usize = 32;

/// How a route is counted (the gateway's route table declares it per row).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateClass {
    /// Ordinary API traffic.
    Standard,
    /// Presents or mints a credential (login, register, PIN, password
    /// reset, refresh).
    Credential,
    /// A long-lived event stream, counted once when it opens.
    Stream,
}

impl RateClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Credential => "credential",
            Self::Stream => "stream",
        }
    }
}

/// At most `limit` requests per `window`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateRule {
    pub limit: u32,
    pub window: Duration,
}

impl RateRule {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self { limit, window }
    }

    pub fn per_second(limit: u32) -> Self {
        Self::new(limit, Duration::from_secs(1))
    }

    pub fn per_minute(limit: u32) -> Self {
        Self::new(limit, Duration::from_secs(60))
    }
}

/// The rule of each class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateClasses {
    pub standard: RateRule,
    pub credential: RateRule,
    pub stream: RateRule,
}

impl RateClasses {
    pub fn rule(&self, class: RateClass) -> RateRule {
        match class {
            RateClass::Standard => self.standard,
            RateClass::Credential => self.credential,
            RateClass::Stream => self.stream,
        }
    }
}

impl Default for RateClasses {
    /// `Standard` 120/s per caller (the gateway's limit before 3.6);
    /// `Credential` 60/min and `Stream` 30/min per caller on top. The
    /// credential rule sits above auth-service's own per-route limits
    /// (login 10/min, confirm 20/min per IP), so it only bites on a flood.
    fn default() -> Self {
        Self {
            standard: RateRule::per_second(120),
            credential: RateRule::per_minute(60),
            stream: RateRule::per_minute(30),
        }
    }
}

struct Bucket {
    window: Duration,
    hits: VecDeque<Instant>,
    last_seen: Instant,
}

impl Bucket {
    fn is_stale(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.last_seen) >= self.window
    }
}

struct Inner {
    buckets: HashMap<String, Bucket>,
    last_sweep: Instant,
}

/// Bounded in-memory sliding-window limiter. Every key keeps its own
/// window, so a sweep never cuts a 15-minute bucket down to a 1-second one.
pub struct RateLimiter {
    inner: Mutex<Inner>,
    max_keys: usize,
}

impl RateLimiter {
    /// Default cap on keys held by one limiter.
    pub const DEFAULT_MAX_KEYS: usize = 100_000;

    pub fn new(max_keys: usize) -> Self {
        Self {
            inner: Mutex::new(Inner {
                buckets: HashMap::new(),
                last_sweep: Instant::now(),
            }),
            max_keys: max_keys.max(1),
        }
    }

    /// Counts one hit for `key` and answers whether it is within `limit`
    /// hits per `window`. A refused hit is not counted.
    pub fn check(&self, key: &str, limit: u32, window: Duration) -> bool {
        self.check_at(key, limit, window, Instant::now())
    }

    /// [`RateLimiter::check`] with a [`RateRule`].
    pub fn check_rule(&self, key: &str, rule: RateRule) -> bool {
        self.check(key, rule.limit, rule.window)
    }

    fn check_at(&self, key: &str, limit: u32, window: Duration, now: Instant) -> bool {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if now.saturating_duration_since(inner.last_sweep) >= SWEEP_INTERVAL {
            sweep(&mut inner.buckets, now);
            inner.last_sweep = now;
        }

        if !inner.buckets.contains_key(key) && inner.buckets.len() >= self.max_keys {
            sweep(&mut inner.buckets, now);
            inner.last_sweep = now;
            if inner.buckets.len() >= self.max_keys {
                evict_least_recent(&mut inner.buckets, self.max_keys);
            }
        }

        let bucket = inner
            .buckets
            .entry(key.to_string())
            .or_insert_with(|| Bucket {
                window,
                hits: VecDeque::new(),
                last_seen: now,
            });
        bucket.window = window;
        bucket.last_seen = now;
        while bucket
            .hits
            .front()
            .is_some_and(|hit| now.saturating_duration_since(*hit) >= window)
        {
            bucket.hits.pop_front();
        }
        let limit = usize::try_from(limit).unwrap_or(usize::MAX);
        if bucket.hits.len() >= limit {
            return false;
        }
        bucket.hits.push_back(now);
        true
    }

    /// Keys currently held (for tests and diagnostics).
    pub fn len(&self) -> usize {
        match self.inner.lock() {
            Ok(guard) => guard.buckets.len(),
            Err(poisoned) => poisoned.into_inner().buckets.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(Self::DEFAULT_MAX_KEYS)
    }
}

fn sweep(buckets: &mut HashMap<String, Bucket>, now: Instant) {
    buckets.retain(|_, bucket| !bucket.is_stale(now));
}

/// Drops the least recently used tenth of the keys (at least one) so the
/// next inserts do not each pay for a full scan.
fn evict_least_recent(buckets: &mut HashMap<String, Bucket>, max_keys: usize) {
    let excess = buckets.len() + 1 - max_keys.min(buckets.len() + 1);
    let target = excess.max(max_keys / 10).max(1).min(buckets.len());
    if target == 0 {
        return;
    }
    let mut by_age: Vec<(Instant, String)> = buckets
        .iter()
        .map(|(key, bucket)| (bucket.last_seen, key.clone()))
        .collect();
    by_age.select_nth_unstable_by(target - 1, |a, b| a.0.cmp(&b.0));
    for (_, key) in by_age.into_iter().take(target) {
        buckets.remove(&key);
    }
}

/// The key for a bearer token: `token:` and a prefix of its SHA-256.
pub fn token_key(token: &str) -> String {
    let digest = hex(&Sha256::digest(token.trim().as_bytes()));
    format!("token:{}", &digest[..TOKEN_KEY_HEX])
}

/// The key for a client address: `ip:<address>` or `ip:unknown`.
pub fn ip_key(client: &ClientIp) -> String {
    format!("ip:{}", client.key())
}

/// The key for a signed-in account.
pub fn account_key(account_id: &str) -> String {
    format!("account:{account_id}")
}

/// Who a request is counted against: its bearer token when it sends one,
/// otherwise its client address.
pub fn caller_key(headers: &HeaderMap, client: &ClientIp) -> String {
    bearer(headers).map_or_else(|| ip_key(client), token_key)
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// Where a [`RateLimitLayer`] counts. The in-memory [`RateLimiter`] is one;
/// a module with a shared store (athene-api's Redis) brings its own.
pub trait RateCheck: Send + Sync {
    /// Counts one hit for `key` under `rule`; whether it is allowed.
    fn check<'a>(&'a self, key: &'a str, rule: RateRule) -> BoxFuture<'a, bool>;
}

impl RateCheck for RateLimiter {
    fn check<'a>(&'a self, key: &'a str, rule: RateRule) -> BoxFuture<'a, bool> {
        let allowed = self.check_rule(key, rule);
        Box::pin(async move { allowed })
    }
}

/// What a [`RateLimitLayer`] refused.
#[derive(Debug, Clone)]
pub struct RateLimited {
    pub class: RateClass,
    pub rule: RateRule,
    /// The caller key, e.g. `ip:203.0.113.9` or `token:<hash prefix>`.
    pub caller: String,
    pub path: String,
}

/// Who a request is counted against (default: [`caller_key`]).
pub type KeyFn = Arc<dyn Fn(&HeaderMap, &ClientIp) -> String + Send + Sync>;

/// Counts every request it sees against its class's rule; refuses with
/// the module's answer once the caller is over it. Reads [`ClientIp`], so
/// it runs inside a [`crate::ClientIpLayer`].
pub struct RateLimitLayer<B> {
    checker: Arc<dyn RateCheck>,
    class: RateClass,
    rule: RateRule,
    key: KeyFn,
    reject: Render<RateLimited, B>,
}

impl<B> Clone for RateLimitLayer<B> {
    fn clone(&self) -> Self {
        Self {
            checker: Arc::clone(&self.checker),
            class: self.class,
            rule: self.rule,
            key: Arc::clone(&self.key),
            reject: Arc::clone(&self.reject),
        }
    }
}

impl<B> RateLimitLayer<B> {
    pub fn new(
        checker: Arc<dyn RateCheck>,
        class: RateClass,
        rule: RateRule,
        reject: Render<RateLimited, B>,
    ) -> Self {
        Self {
            checker,
            class,
            rule,
            key: Arc::new(caller_key),
            reject,
        }
    }

    /// Counts by `key` instead of [`caller_key`].
    #[must_use]
    pub fn keyed_by(mut self, key: KeyFn) -> Self {
        self.key = key;
        self
    }
}

impl<S, B> Layer<S> for RateLimitLayer<B> {
    type Service = RateLimitService<S, B>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            layer: self.clone(),
        }
    }
}

pub struct RateLimitService<S, B> {
    inner: S,
    layer: RateLimitLayer<B>,
}

impl<S: Clone, B> Clone for RateLimitService<S, B> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            layer: self.layer.clone(),
        }
    }
}

impl<S, ReqB, ResB> Service<Request<ReqB>> for RateLimitService<S, ResB>
where
    S: Service<Request<ReqB>, Response = Response<ResB>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
    ReqB: Send + 'static,
    ResB: Send + 'static,
{
    type Response = Response<ResB>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Response<ResB>, S::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<ReqB>) -> Self::Future {
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let layer = self.layer.clone();
        Box::pin(async move {
            let client = ClientIp::from_extensions(request.extensions());
            let caller = (layer.key)(request.headers(), &client);
            let key = format!("{}:{caller}", layer.class.as_str());
            if layer.checker.check(&key, layer.rule).await {
                return inner.call(request).await;
            }
            let refused = RateLimited {
                class: layer.class,
                rule: layer.rule,
                caller,
                path: request.uri().path().to_string(),
            };
            tracing::warn!(
                event = "security.rate_limited",
                class = refused.class.as_str(),
                limit = refused.rule.limit,
                window_ms = u64::try_from(refused.rule.window.as_millis()).unwrap_or(u64::MAX),
                caller = %refused.caller,
                path = %refused.path,
            );
            Ok((layer.reject)(&refused))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;
    use tower::ServiceExt;

    #[test]
    fn allows_up_to_the_limit_then_refuses() {
        let limiter = RateLimiter::new(100);
        let now = Instant::now();
        let window = Duration::from_secs(60);
        for _ in 0..3 {
            assert!(limiter.check_at("k", 3, window, now));
        }
        assert!(!limiter.check_at("k", 3, window, now));
        assert!(limiter.check_at("other", 3, window, now));
    }

    #[test]
    fn hits_expire_with_the_window() {
        let limiter = RateLimiter::new(100);
        let now = Instant::now();
        let window = Duration::from_secs(1);
        assert!(limiter.check_at("k", 1, window, now));
        assert!(!limiter.check_at("k", 1, window, now));
        assert!(limiter.check_at("k", 1, window, now + Duration::from_millis(1_001)));
    }

    #[test]
    fn a_sweep_keeps_long_windows_intact() {
        let limiter = RateLimiter::new(100);
        let start = Instant::now();
        let long = Duration::from_secs(15 * 60);
        assert!(limiter.check_at("sensitive", 1, long, start));
        let later = start + SWEEP_INTERVAL + Duration::from_secs(5);
        assert!(limiter.check_at("global", 100, Duration::from_secs(1), later));
        assert!(!limiter.check_at("sensitive", 1, long, later));
    }

    #[test]
    fn the_number_of_keys_is_bounded() {
        let limiter = RateLimiter::new(50);
        let now = Instant::now();
        for i in 0..10_000 {
            limiter.check_at(&format!("spoofed-{i}"), 10, Duration::from_secs(60), now);
        }
        assert!(limiter.len() <= 50, "held {} keys", limiter.len());
    }

    #[test]
    fn eviction_prefers_the_least_recently_used_keys() {
        let limiter = RateLimiter::new(10);
        let start = Instant::now();
        let window = Duration::from_secs(600);
        assert!(limiter.check_at("hot", 1, window, start));
        for i in 0..9 {
            limiter.check_at(&format!("cold-{i}"), 5, window, start);
        }
        let later = start + Duration::from_secs(1);
        assert!(!limiter.check_at("hot", 1, window, later));
        limiter.check_at("new", 5, window, later);
        assert!(!limiter.check_at("hot", 1, window, later));
    }

    #[test]
    fn stale_keys_are_dropped_before_live_ones() {
        let limiter = RateLimiter::new(2);
        let start = Instant::now();
        assert!(limiter.check_at("live", 1, Duration::from_secs(600), start));
        limiter.check_at("stale", 1, Duration::from_secs(1), start);
        let later = start + Duration::from_secs(5);
        limiter.check_at("new", 1, Duration::from_secs(600), later);
        assert!(!limiter.check_at("live", 1, Duration::from_secs(600), later));
        assert_eq!(limiter.len(), 2);
    }

    #[test]
    fn keys_never_hold_the_token() {
        let client = ClientIp(Some("127.0.0.1".parse().unwrap()));
        let mut alice = HeaderMap::new();
        alice.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer token-alice"),
        );
        let mut bob = HeaderMap::new();
        bob.insert(AUTHORIZATION, HeaderValue::from_static("Bearer token-bob"));
        let alice_key = caller_key(&alice, &client);
        assert!(alice_key.starts_with("token:"), "{alice_key}");
        assert_eq!(alice_key.len(), "token:".len() + TOKEN_KEY_HEX);
        assert!(!alice_key.contains("alice"));
        assert_ne!(alice_key, caller_key(&bob, &client));
        assert_eq!(caller_key(&HeaderMap::new(), &client), "ip:127.0.0.1");
        assert_eq!(account_key("u-1"), "account:u-1");
    }

    fn layer(
        class: RateClass,
        rule: RateRule,
        limiter: Arc<RateLimiter>,
    ) -> RateLimitLayer<String> {
        RateLimitLayer::new(
            limiter,
            class,
            rule,
            Arc::new(|refused: &RateLimited| {
                let mut response = Response::new(format!("slow down: {}", refused.class.as_str()));
                *response.status_mut() = http::StatusCode::TOO_MANY_REQUESTS;
                response
            }),
        )
    }

    async fn status<S>(service: &S, client: &str, token: Option<&str>) -> u16
    where
        S: Service<Request<()>, Response = Response<String>, Error = std::convert::Infallible>
            + Clone,
    {
        let mut request = Request::new(());
        request
            .extensions_mut()
            .insert(ClientIp(Some(client.parse().unwrap())));
        if let Some(token) = token {
            request.headers_mut().insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
            );
        }
        service
            .clone()
            .oneshot(request)
            .await
            .unwrap()
            .status()
            .as_u16()
    }

    fn ok(
        _: Request<()>,
    ) -> std::future::Ready<Result<Response<String>, std::convert::Infallible>> {
        std::future::ready(Ok(Response::new("ok".to_string())))
    }

    #[tokio::test]
    async fn the_layer_counts_each_caller_and_class_apart() {
        let limiter = Arc::new(RateLimiter::new(1000));
        let inner = tower::service_fn(ok);
        let standard = layer(
            RateClass::Standard,
            RateRule::per_minute(2),
            Arc::clone(&limiter),
        )
        .layer(inner);
        assert_eq!(status(&standard, "203.0.113.1", None).await, 200);
        assert_eq!(status(&standard, "203.0.113.1", None).await, 200);
        assert_eq!(status(&standard, "203.0.113.1", None).await, 429);
        // Another address, and a signed-in caller behind the same one, are
        // counted on their own.
        assert_eq!(status(&standard, "203.0.113.2", None).await, 200);
        assert_eq!(status(&standard, "203.0.113.1", Some("a-token")).await, 200);

        // The credential class has its own buckets.
        let credential = layer(
            RateClass::Credential,
            RateRule::per_minute(1),
            Arc::clone(&limiter),
        )
        .layer(inner);
        assert_eq!(status(&credential, "203.0.113.1", None).await, 200);
        assert_eq!(status(&credential, "203.0.113.1", None).await, 429);
    }
}
