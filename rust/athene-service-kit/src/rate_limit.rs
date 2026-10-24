//! Rate limits: one bounded limiter, keys by IP, token or account, a layer
//! per route class, and a cap on concurrently open streams (Q-044, SEC-19,
//! SEC-03; 3.6b).
//!
//! [`RateLimiter`] (moved from auth-service `security/rate_limiter.rs`)
//! keeps a sliding window per key. The number of keys is capped: at the cap,
//! stale keys go first and then the least recently used, so a flood of fresh
//! keys (random `X-Forwarded-For`, random tokens) costs a bounded amount of
//! memory instead of growing without limit — the gateway's old map never
//! forgot a key (SEC-19).
//!
//! **Keys.** A key never holds a credential: a token is counted under a
//! prefix of its SHA-256 ([`token_key`]). A bearer only earns its own
//! bucket once the service has **verified** it ([`VerifiedCallers`]):
//! counted per token, the users behind one proxy address no longer share
//! one bucket (the `global:127.0.0.1` bucket that held every user, SEC-03);
//! counted per *unverified* token, a client that sends a fresh random
//! bearer with every request gets a fresh bucket every time and no limit
//! holds (VERIFY_3_6 S1). So [`RateLimitLayer`] counts by client address
//! unless it is told otherwise ([`RateLimitLayer::keyed_by`]); the
//! gateway counts `Credential` by address only and `Standard` by a verified
//! bearer, else the address.
//!
//! [`RateLimitLayer`] counts every request of one [`RateClass`] — the
//! classes the gateway's route table declares (step 3.2): `Standard` for
//! all traffic, `Credential` and `Stream` on top for the routes that carry
//! them (`route_layer`), each with its own [`RateRule`]. A refusal carries
//! how long until the caller's next request would pass
//! ([`RateLimited::retry_after`]) for `Retry-After`.
//!
//! [`ConcurrencyLimitLayer`] caps how many responses of one class a caller
//! holds open at once — for event streams, which live for minutes and are
//! opened on every page load: a burst of reloads is fine, a pile-up of open
//! streams is not. The slot travels with the response body ([`SlotBody`])
//! and is released when the body ends or is dropped, which is also what
//! happens when the client disconnects.

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
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
    /// A long-lived event stream, counted once when it opens, and capped
    /// in how many a caller holds open at once ([`ConcurrencyLimitLayer`]).
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
    /// `Credential` 60/min and `Stream` 300/min per caller on top. The
    /// credential rule sits above auth-service's own per-route limits
    /// (login 10/min, confirm 20/min per IP), so it only bites on a flood.
    /// The stream rule only catches a reconnect storm: every page load
    /// opens two streams, and every local client shares `127.0.0.1` (30/min
    /// was 15 page loads a minute for the whole machine, DEPLOY_3_6). What
    /// bounds streams is [`DEFAULT_STREAMS_PER_CALLER`] open at once.
    fn default() -> Self {
        Self {
            standard: RateRule::per_second(120),
            credential: RateRule::per_minute(60),
            stream: RateRule::per_minute(300),
        }
    }
}

/// Event streams one caller may hold open at once (the gateway's
/// `ATHENE_API_RATE_LIMIT_STREAM_CONCURRENT`): eight tabs with their two
/// streams each.
pub const DEFAULT_STREAMS_PER_CALLER: u32 = 16;

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

    /// How long until `key` may make its next request under `rule`: zero
    /// when it may now, else until the hit that holds its place expires.
    pub fn retry_after(&self, key: &str, rule: RateRule) -> Duration {
        self.retry_after_at(key, rule.limit, rule.window, Instant::now())
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn check_at(&self, key: &str, limit: u32, window: Duration, now: Instant) -> bool {
        let mut inner = self.lock();

        if now.saturating_duration_since(inner.last_sweep) >= SWEEP_INTERVAL {
            sweep(&mut inner.buckets, now);
            inner.last_sweep = now;
        }

        if !inner.buckets.contains_key(key) && inner.buckets.len() >= self.max_keys {
            sweep(&mut inner.buckets, now);
            inner.last_sweep = now;
            if inner.buckets.len() >= self.max_keys {
                evict_least_recent(&mut inner.buckets, self.max_keys, |bucket| bucket.last_seen);
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

    fn retry_after_at(&self, key: &str, limit: u32, window: Duration, now: Instant) -> Duration {
        let inner = self.lock();
        let Some(bucket) = inner.buckets.get(key) else {
            return Duration::ZERO;
        };
        let live: Vec<Instant> = bucket
            .hits
            .iter()
            .copied()
            .filter(|hit| now.saturating_duration_since(*hit) < window)
            .collect();
        let limit = usize::try_from(limit).unwrap_or(usize::MAX);
        if live.len() < limit {
            return Duration::ZERO;
        }
        // The caller is back under the limit when all but `limit - 1` of
        // its live hits have expired: the oldest `live - limit + 1` of them.
        live.get(live.len() - limit).map_or(window, |hit| {
            window.saturating_sub(now.saturating_duration_since(*hit))
        })
    }

    /// Keys currently held (for tests and diagnostics).
    pub fn len(&self) -> usize {
        self.lock().buckets.len()
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
fn evict_least_recent<V>(
    entries: &mut HashMap<String, V>,
    max_keys: usize,
    last_used: impl Fn(&V) -> Instant,
) {
    let excess = entries.len() + 1 - max_keys.min(entries.len() + 1);
    let target = excess.max(max_keys / 10).max(1).min(entries.len());
    if target == 0 {
        return;
    }
    let mut by_age: Vec<(Instant, String)> = entries
        .iter()
        .map(|(key, value)| (last_used(value), key.clone()))
        .collect();
    by_age.select_nth_unstable_by(target - 1, |a, b| a.0.cmp(&b.0));
    for (_, key) in by_age.into_iter().take(target) {
        entries.remove(&key);
    }
}

/// The bearers a service has verified, so each may count on its own
/// (S1, 3.6b): a bounded set of [`token_key`]s, each good for `ttl` after
/// the last time it was verified.
///
/// The gateway marks a bearer once auth-service accepted it (`/auth/me`
/// succeeded) and forgets it once auth-service refused it. A bearer that
/// was never verified counts against the client address, so random
/// bearers buy nothing. A verification is renewed with every request that
/// checks the bearer again, and an entry that is not renewed expires; at
/// the cap the expired go first, then the least recently verified tenth.
pub struct VerifiedCallers {
    verified: Mutex<HashMap<String, Instant>>,
    ttl: Duration,
    max_keys: usize,
}

impl VerifiedCallers {
    /// How long a verification counts without being renewed.
    pub const DEFAULT_TTL: Duration = Duration::from_secs(10 * 60);

    pub fn new(max_keys: usize, ttl: Duration) -> Self {
        Self {
            verified: Mutex::new(HashMap::new()),
            ttl,
            max_keys: max_keys.max(1),
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<String, Instant>> {
        match self.verified.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// `token` was verified just now.
    pub fn mark(&self, token: &str) {
        self.mark_at(token, Instant::now());
    }

    fn mark_at(&self, token: &str, now: Instant) {
        let key = token_key(token);
        let mut verified = self.lock();
        if !verified.contains_key(&key) && verified.len() >= self.max_keys {
            let ttl = self.ttl;
            verified.retain(|_, at| now.saturating_duration_since(*at) < ttl);
            if verified.len() >= self.max_keys {
                evict_least_recent(&mut verified, self.max_keys, |at| *at);
            }
        }
        verified.insert(key, now);
    }

    /// `token` was refused: it counts against its address again.
    pub fn forget(&self, token: &str) {
        self.lock().remove(&token_key(token));
    }

    pub fn is_verified(&self, token: &str) -> bool {
        self.is_verified_at(token, Instant::now())
    }

    fn is_verified_at(&self, token: &str, now: Instant) -> bool {
        self.lock()
            .get(&token_key(token))
            .is_some_and(|at| now.saturating_duration_since(*at) < self.ttl)
    }

    /// The key to count a request under: its bearer's [`token_key`] when
    /// that bearer is verified, else the client address ([`ip_key`]).
    pub fn key(&self, token: Option<&str>, client: &ClientIp) -> String {
        match token.map(str::trim).filter(|token| !token.is_empty()) {
            Some(token) if self.is_verified(token) => token_key(token),
            _ => ip_key(client),
        }
    }

    /// Bearers currently held (for tests and diagnostics).
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for VerifiedCallers {
    fn default() -> Self {
        Self::new(RateLimiter::DEFAULT_MAX_KEYS, Self::DEFAULT_TTL)
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

/// Who a request is counted against: its client address. The default of
/// [`RateLimitLayer`] and [`ConcurrencyLimitLayer`].
pub fn client_key(_: &HeaderMap, client: &ClientIp) -> String {
    ip_key(client)
}

/// Who a request is counted against: its bearer token when it sends one,
/// otherwise its client address.
///
/// **The bearer is not checked**, so a caller picks its own bucket by
/// sending a new one (S1). Use it only where another limit by address
/// holds as well (auth-service checks `global_ip` after it); otherwise
/// count by [`client_key`] or by [`VerifiedCallers::key`].
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

    /// After a refusal: how long until `key` may try again, for
    /// `Retry-After`. The whole window unless the store knows better (a
    /// fixed window can reset no later than that).
    fn retry_after(&self, _key: &str, rule: RateRule) -> Duration {
        rule.window
    }
}

impl RateCheck for RateLimiter {
    fn check<'a>(&'a self, key: &'a str, rule: RateRule) -> BoxFuture<'a, bool> {
        let allowed = self.check_rule(key, rule);
        Box::pin(async move { allowed })
    }

    fn retry_after(&self, key: &str, rule: RateRule) -> Duration {
        RateLimiter::retry_after(self, key, rule)
    }
}

/// Whole seconds for a `Retry-After` header: rounded up, at least 1.
pub fn retry_after_secs(wait: Duration) -> u64 {
    let secs = wait.as_secs();
    let secs = if wait.subsec_nanos() > 0 {
        secs.saturating_add(1)
    } else {
        secs
    };
    secs.max(1)
}

/// What a [`RateLimitLayer`] refused.
#[derive(Debug, Clone)]
pub struct RateLimited {
    pub class: RateClass,
    pub rule: RateRule,
    /// The caller key, e.g. `ip:203.0.113.9` or `token:<hash prefix>`.
    pub caller: String,
    pub path: String,
    /// How long until the caller's next request would pass.
    pub retry_after: Duration,
}

impl RateLimited {
    /// [`Self::retry_after`] for the `Retry-After` header.
    pub fn retry_after_secs(&self) -> u64 {
        retry_after_secs(self.retry_after)
    }
}

/// Who a request is counted against (default: [`client_key`]).
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
            key: Arc::new(client_key),
            reject,
        }
    }

    /// Counts by `key` instead of [`client_key`].
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
                retry_after: layer.checker.retry_after(&key, layer.rule),
            };
            tracing::warn!(
                event = "security.rate_limited",
                class = refused.class.as_str(),
                limit = refused.rule.limit,
                window_ms = millis(refused.rule.window),
                retry_after_ms = millis(refused.retry_after),
                caller = %refused.caller,
                path = %crate::observability::redact::path(&refused.path),
            );
            Ok((layer.reject)(&refused))
        })
    }
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

// ------------------------------------------------------------------
// Concurrently open responses (event streams), 3.6b
// ------------------------------------------------------------------

/// How many responses each caller holds open: a count per key, bounded in
/// keys. A key exists only while it holds a slot, so the map is as large as
/// the number of distinct callers with an open stream.
pub struct ConcurrencyLimiter {
    open: Mutex<HashMap<String, u32>>,
    max_keys: usize,
}

impl ConcurrencyLimiter {
    pub fn new(max_keys: usize) -> Self {
        Self {
            open: Mutex::new(HashMap::new()),
            max_keys: max_keys.max(1),
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<String, u32>> {
        match self.open.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// A slot for `key` while it holds fewer than `limit`; `None` when it
    /// holds `limit` already, or when `max_keys` other callers hold slots
    /// (an open slot cannot be evicted, so the cap refuses new callers
    /// instead: memory stays bounded). The slot is released when dropped.
    pub fn try_acquire(self: &Arc<Self>, key: &str, limit: u32) -> Option<Slot> {
        let mut open = self.lock();
        let held = open.get(key).copied();
        if held.is_none() && open.len() >= self.max_keys {
            return None;
        }
        let held = held.unwrap_or(0);
        if held >= limit {
            return None;
        }
        open.insert(key.to_string(), held.saturating_add(1));
        Some(Slot {
            limiter: Arc::clone(self),
            key: key.to_string(),
        })
    }

    fn release(&self, key: &str) {
        let mut open = self.lock();
        match open.get_mut(key) {
            Some(held) if *held > 1 => *held -= 1,
            Some(_) => {
                open.remove(key);
            }
            None => {}
        }
    }

    /// Slots `key` holds now.
    pub fn open(&self, key: &str) -> u32 {
        self.lock().get(key).copied().unwrap_or(0)
    }

    /// Slots held by everyone.
    pub fn total(&self) -> u64 {
        self.lock().values().map(|held| u64::from(*held)).sum()
    }

    /// Callers holding slots.
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for ConcurrencyLimiter {
    fn default() -> Self {
        Self::new(RateLimiter::DEFAULT_MAX_KEYS)
    }
}

/// One held place of a [`ConcurrencyLimiter`]; dropping it frees it.
pub struct Slot {
    limiter: Arc<ConcurrencyLimiter>,
    key: String,
}

impl fmt::Debug for Slot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Slot").field("key", &self.key).finish()
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.limiter.release(&self.key);
    }
}

/// A response body that holds a [`Slot`] for as long as it is being sent.
///
/// The slot is released when the body ends, or when the server drops the
/// body unfinished — which is what hyper does when the client goes away,
/// so a closed tab or a reload frees its stream's place.
pub struct SlotBody<B> {
    inner: B,
    slot: Option<Slot>,
}

impl<B> SlotBody<B> {
    pub fn new(inner: B, slot: Option<Slot>) -> Self {
        Self { inner, slot }
    }

    /// Whether the body still holds its slot.
    pub fn holds_slot(&self) -> bool {
        self.slot.is_some()
    }
}

impl<B> http_body::Body for SlotBody<B>
where
    B: http_body::Body + Unpin,
{
    type Data = B::Data;
    type Error = B::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        let polled = Pin::new(&mut this.inner).poll_frame(cx);
        if matches!(polled, Poll::Ready(None)) {
            this.slot = None;
        }
        polled
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        self.inner.size_hint()
    }
}

/// What a [`ConcurrencyLimitLayer`] refused.
#[derive(Debug, Clone)]
pub struct ConcurrencyLimited {
    pub class: RateClass,
    /// Responses the caller may hold open at once.
    pub limit: u32,
    /// The caller key, e.g. `ip:203.0.113.9`.
    pub caller: String,
    pub path: String,
    /// The hint for `Retry-After`: a slot frees whenever one of the
    /// caller's streams closes, which no one can predict.
    pub retry_after: Duration,
}

impl ConcurrencyLimited {
    /// [`Self::retry_after`] for the `Retry-After` header.
    pub fn retry_after_secs(&self) -> u64 {
        retry_after_secs(self.retry_after)
    }
}

/// Lets each caller hold at most `limit` responses of its class open at
/// once; refuses the next with the module's answer. The response body
/// carries the slot ([`SlotBody`]). Reads [`ClientIp`], so it runs inside
/// a [`crate::ClientIpLayer`].
pub struct ConcurrencyLimitLayer<B> {
    limiter: Arc<ConcurrencyLimiter>,
    class: RateClass,
    limit: u32,
    key: KeyFn,
    retry_after: Duration,
    reject: Render<ConcurrencyLimited, B>,
}

impl<B> Clone for ConcurrencyLimitLayer<B> {
    fn clone(&self) -> Self {
        Self {
            limiter: Arc::clone(&self.limiter),
            class: self.class,
            limit: self.limit,
            key: Arc::clone(&self.key),
            retry_after: self.retry_after,
            reject: Arc::clone(&self.reject),
        }
    }
}

impl<B> ConcurrencyLimitLayer<B> {
    /// Default `Retry-After` of a refusal.
    pub const DEFAULT_RETRY_AFTER: Duration = Duration::from_secs(5);

    /// Counted by [`client_key`].
    pub fn new(
        limiter: Arc<ConcurrencyLimiter>,
        class: RateClass,
        limit: u32,
        reject: Render<ConcurrencyLimited, B>,
    ) -> Self {
        Self {
            limiter,
            class,
            limit,
            key: Arc::new(client_key),
            retry_after: Self::DEFAULT_RETRY_AFTER,
            reject,
        }
    }

    /// Counts by `key` instead of [`client_key`].
    #[must_use]
    pub fn keyed_by(mut self, key: KeyFn) -> Self {
        self.key = key;
        self
    }

    /// The `Retry-After` hint of a refusal.
    #[must_use]
    pub fn retry_after(mut self, wait: Duration) -> Self {
        self.retry_after = wait;
        self
    }
}

impl<S, B> Layer<S> for ConcurrencyLimitLayer<B> {
    type Service = ConcurrencyLimitService<S, B>;

    fn layer(&self, inner: S) -> Self::Service {
        ConcurrencyLimitService {
            inner,
            layer: self.clone(),
        }
    }
}

pub struct ConcurrencyLimitService<S, B> {
    inner: S,
    layer: ConcurrencyLimitLayer<B>,
}

impl<S: Clone, B> Clone for ConcurrencyLimitService<S, B> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            layer: self.layer.clone(),
        }
    }
}

impl<S, ReqB, ResB> Service<Request<ReqB>> for ConcurrencyLimitService<S, ResB>
where
    S: Service<Request<ReqB>, Response = Response<ResB>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
    ReqB: Send + 'static,
    ResB: Send + 'static,
{
    type Response = Response<SlotBody<ResB>>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, S::Error>>;

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
            let Some(slot) = layer.limiter.try_acquire(&key, layer.limit) else {
                let refused = ConcurrencyLimited {
                    class: layer.class,
                    limit: layer.limit,
                    caller,
                    path: request.uri().path().to_string(),
                    retry_after: layer.retry_after,
                };
                tracing::warn!(
                    event = "security.concurrency_limited",
                    class = refused.class.as_str(),
                    limit = refused.limit,
                    open = layer.limiter.open(&key),
                    retry_after_ms = millis(refused.retry_after),
                    caller = %refused.caller,
                    path = %crate::observability::redact::path(&refused.path),
                );
                return Ok((layer.reject)(&refused).map(|body| SlotBody::new(body, None)));
            };
            // An error or a response that is not sent drops the slot with it.
            let response = inner.call(request).await?;
            Ok(response.map(|body| SlotBody::new(body, Some(slot))))
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
        // Another address is counted on its own.
        assert_eq!(status(&standard, "203.0.113.2", None).await, 200);

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

    /// S1 (3.6b): by default a bearer buys no bucket of its own — a fresh
    /// random bearer per request is counted against the address like no
    /// bearer at all.
    #[tokio::test]
    async fn random_bearers_do_not_escape_the_address_bucket() {
        let limiter = Arc::new(RateLimiter::new(1000));
        let credential = layer(
            RateClass::Credential,
            RateRule::per_minute(60),
            Arc::clone(&limiter),
        )
        .layer(tower::service_fn(ok));
        let mut refused = 0;
        for attempt in 0..100 {
            let bearer = format!("random-bearer-{attempt}-{}", uuid::Uuid::new_v4());
            if status(&credential, "203.0.113.7", Some(&bearer)).await == 429 {
                refused += 1;
            }
        }
        assert_eq!(refused, 40, "60 of 100 pass, then the address is over");
        assert_eq!(limiter.len(), 1, "one bucket, the address's");
    }

    /// Only a bearer the service verified counts on its own; anyone else
    /// behind the same address shares the address bucket.
    #[tokio::test]
    async fn verified_bearers_count_on_their_own() {
        let limiter = Arc::new(RateLimiter::new(1000));
        let verified = Arc::new(VerifiedCallers::default());
        let keyed = Arc::clone(&verified);
        let standard = layer(
            RateClass::Standard,
            RateRule::per_minute(2),
            Arc::clone(&limiter),
        )
        .keyed_by(Arc::new(move |headers, client| {
            keyed.key(bearer(headers), client)
        }))
        .layer(tower::service_fn(ok));
        verified.mark("alice-token");
        verified.mark("bob-token");
        for token in ["alice-token", "bob-token"] {
            assert_eq!(status(&standard, "203.0.113.1", Some(token)).await, 200);
            assert_eq!(status(&standard, "203.0.113.1", Some(token)).await, 200);
            assert_eq!(status(&standard, "203.0.113.1", Some(token)).await, 429);
        }
        // The address bucket is untouched by them, and unverified bearers
        // share it.
        assert_eq!(
            status(&standard, "203.0.113.1", Some("forged-1")).await,
            200
        );
        assert_eq!(
            status(&standard, "203.0.113.1", Some("forged-2")).await,
            200
        );
        assert_eq!(status(&standard, "203.0.113.1", None).await, 429);
        // A bearer that was refused counts against the address again.
        verified.forget("alice-token");
        assert_eq!(
            status(&standard, "203.0.113.1", Some("alice-token")).await,
            429
        );
    }

    #[test]
    fn a_verification_expires_and_the_set_is_bounded() {
        let verified = VerifiedCallers::new(10, Duration::from_secs(60));
        let start = Instant::now();
        verified.mark_at("t", start);
        assert!(verified.is_verified_at("t", start + Duration::from_secs(59)));
        assert!(!verified.is_verified_at("t", start + Duration::from_secs(60)));
        let client = ClientIp(Some("203.0.113.9".parse().unwrap()));
        assert_eq!(verified.key(None, &client), "ip:203.0.113.9");
        assert_eq!(verified.key(Some("  "), &client), "ip:203.0.113.9");
        assert_eq!(verified.key(Some("unknown"), &client), "ip:203.0.113.9");
        verified.mark("t");
        assert_eq!(verified.key(Some("t"), &client), token_key("t"));
        for i in 0..1_000 {
            verified.mark_at(&format!("flood-{i}"), start + Duration::from_millis(i));
        }
        assert!(verified.len() <= 10, "held {}", verified.len());
    }

    #[test]
    fn retry_after_waits_for_the_hit_that_frees_a_place() {
        let limiter = RateLimiter::new(100);
        let start = Instant::now();
        let window = Duration::from_secs(60);
        assert!(limiter.check_at("k", 2, window, start));
        assert!(limiter.check_at("k", 2, window, start + Duration::from_secs(10)));
        let now = start + Duration::from_secs(20);
        assert!(!limiter.check_at("k", 2, window, now));
        assert_eq!(
            limiter.retry_after_at("k", 2, window, now),
            Duration::from_secs(40)
        );
        assert_eq!(
            limiter.retry_after_at("other", 2, window, now),
            Duration::ZERO
        );
        assert_eq!(
            limiter.retry_after_at("k", 2, window, start + Duration::from_secs(61)),
            Duration::ZERO
        );
        assert_eq!(retry_after_secs(Duration::from_millis(1_200)), 2);
        assert_eq!(retry_after_secs(Duration::from_secs(40)), 40);
        assert_eq!(retry_after_secs(Duration::ZERO), 1);
    }

    #[tokio::test]
    async fn a_refusal_carries_its_retry_after() {
        let limiter = Arc::new(RateLimiter::new(100));
        let service = RateLimitLayer::new(
            limiter,
            RateClass::Credential,
            RateRule::per_minute(1),
            Arc::new(|refused: &RateLimited| {
                let mut response = Response::new(refused.retry_after_secs().to_string());
                *response.status_mut() = http::StatusCode::TOO_MANY_REQUESTS;
                response
            }),
        )
        .layer(tower::service_fn(ok));
        assert_eq!(status(&service, "203.0.113.1", None).await, 200);
        let mut request = Request::new(());
        request
            .extensions_mut()
            .insert(ClientIp(Some("203.0.113.1".parse().unwrap())));
        let refused = service.oneshot(request).await.unwrap();
        assert_eq!(refused.status(), http::StatusCode::TOO_MANY_REQUESTS);
        let secs: u64 = refused.body().parse().unwrap();
        assert!((59..=60).contains(&secs), "{secs}");
    }

    #[test]
    fn slots_are_counted_per_key_and_released_on_drop() {
        let limiter = Arc::new(ConcurrencyLimiter::new(100));
        let first = limiter.try_acquire("k", 2).unwrap();
        let second = limiter.try_acquire("k", 2).unwrap();
        assert!(limiter.try_acquire("k", 2).is_none());
        assert!(limiter.try_acquire("other", 2).is_some());
        assert_eq!(limiter.open("k"), 2);
        drop(first);
        assert_eq!(limiter.open("k"), 1);
        let third = limiter.try_acquire("k", 2).unwrap();
        drop(second);
        drop(third);
        assert_eq!(limiter.open("k"), 0);
        assert!(limiter.is_empty(), "a key without slots is gone");
        assert!(limiter.try_acquire("zero", 0).is_none());
    }

    #[test]
    fn the_number_of_callers_holding_slots_is_bounded() {
        let limiter = Arc::new(ConcurrencyLimiter::new(3));
        let held: Vec<Slot> = (0..3)
            .map(|i| limiter.try_acquire(&format!("caller-{i}"), 4).unwrap())
            .collect();
        assert!(limiter.try_acquire("caller-new", 4).is_none());
        // A caller already in holds on to its own limit.
        let more = limiter.try_acquire("caller-0", 4);
        assert!(more.is_some());
        assert_eq!(limiter.total(), 4);
        drop(more);
        drop(held);
        assert!(limiter.try_acquire("caller-new", 4).is_some());
    }

    fn streaming(
        limiter: &Arc<ConcurrencyLimiter>,
        limit: u32,
    ) -> impl Service<
        Request<()>,
        Response = Response<SlotBody<String>>,
        Error = std::convert::Infallible,
    > + Clone {
        ConcurrencyLimitLayer::new(
            Arc::clone(limiter),
            RateClass::Stream,
            limit,
            Arc::new(|refused: &ConcurrencyLimited| {
                let mut response = Response::new(format!(
                    "busy {} {}",
                    refused.limit,
                    refused.retry_after_secs()
                ));
                *response.status_mut() = http::StatusCode::TOO_MANY_REQUESTS;
                response
            }),
        )
        .layer(tower::service_fn(ok))
    }

    async fn open<S>(service: &S, client: &str) -> Response<SlotBody<String>>
    where
        S: Service<
                Request<()>,
                Response = Response<SlotBody<String>>,
                Error = std::convert::Infallible,
            > + Clone,
    {
        let mut request = Request::new(());
        request
            .extensions_mut()
            .insert(ClientIp(Some(client.parse().unwrap())));
        service.clone().oneshot(request).await.unwrap()
    }

    /// 3.6b: streams are limited by how many are open, not only by how
    /// often they open; a slot lives exactly as long as its response body.
    #[tokio::test]
    async fn the_layer_holds_a_slot_for_as_long_as_the_body_lives() {
        use http_body_util::BodyExt;

        let limiter = Arc::new(ConcurrencyLimiter::new(100));
        let service = streaming(&limiter, 2);
        let first = open(&service, "203.0.113.1").await;
        let second = open(&service, "203.0.113.1").await;
        assert!(first.body().holds_slot() && second.body().holds_slot());
        assert_eq!(limiter.open("stream:ip:203.0.113.1"), 2);

        let refused = open(&service, "203.0.113.1").await;
        assert_eq!(refused.status(), http::StatusCode::TOO_MANY_REQUESTS);
        assert!(!refused.body().holds_slot());
        let text = refused.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&text[..], b"busy 2 5");
        // Another address has its own slots.
        let elsewhere = open(&service, "203.0.113.2").await;
        assert_eq!(elsewhere.status(), http::StatusCode::OK);

        // A body dropped unfinished (the client went away) frees its slot.
        drop(first);
        assert_eq!(limiter.open("stream:ip:203.0.113.1"), 1);
        // A body sent to its end frees it as well, before it is dropped.
        let mut body = second.into_body();
        while body.frame().await.is_some() {}
        assert!(!body.holds_slot());
        assert_eq!(limiter.open("stream:ip:203.0.113.1"), 0);
        let again = open(&service, "203.0.113.1").await;
        assert_eq!(again.status(), http::StatusCode::OK);
        drop((again, elsewhere, body));
        assert!(limiter.is_empty());
    }
}
