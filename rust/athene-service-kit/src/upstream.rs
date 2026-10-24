//! The client to the Fenrir host gateway (`FENRIR_GATEWAY_ENDPOINT`), the
//! one way the services call each other (BE-07, BE-08, BE-14).
//!
//! A call is the host's envelope — `{target, verb, path, headers, query,
//! body, timeout_ms}` posted to the endpoint — and the answer is the
//! target's `{status, audit_id, headers, body}` or the host's own
//! `{status, error}`. Around it:
//!
//! - **Budget.** Each attempt has `attempt_timeout` (also sent to the host
//!   as its per-attempt timeout); an optional `budget` caps the whole call,
//!   retries and back-off included, so the inner chain can no longer run
//!   longer than the caller waits (BE-08).
//! - **Retries** only for idempotent methods (GET, HEAD, OPTIONS, PUT,
//!   DELETE) and only on infrastructure failures: no connection, a timeout,
//!   an unreadable answer, a 408/5xx from the host — and, where the caller
//!   asks for it, a 5xx from the target. Never a 4xx, never a 429: a retry
//!   only adds to the load that caused it (SEC-03). POST and PATCH are
//!   never retried here (BE-08).
//! - **A breaker per target** (or per target and route family): closed →
//!   open after `breaker_threshold` consecutive infrastructure failures →
//!   half-open after the cooldown, when one probe goes through and every
//!   other call still fails fast. A 4xx (429 above all) is an answer from a
//!   working target and closes it.
//! - **Error classes:** [`UpstreamError::class`] maps every failure onto
//!   [`ErrorClass`]; each service answers the class its own way.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error_class::ErrorClass;
use crate::util::excerpt;

/// How the client behaves. `Duration::ZERO` means "not set" for the
/// timeouts, as `0` did in the modules' env settings.
#[derive(Debug, Clone)]
pub struct UpstreamConfig {
    /// The host gateway's `/call` URL.
    pub endpoint: String,
    /// Per attempt; also the host's per-attempt timeout.
    pub attempt_timeout: Duration,
    pub connect_timeout: Duration,
    /// The whole call, retries and back-off included. `None`: attempts ×
    /// `attempt_timeout` plus back-off, as before 3.6.
    pub budget: Option<Duration>,
    /// Retries after the first attempt (idempotent methods only).
    pub retry_attempts: u32,
    pub retry_base_delay: Duration,
    pub retry_max_delay: Duration,
    /// Consecutive infrastructure failures that open a breaker; `0` turns
    /// breakers off.
    pub breaker_threshold: u32,
    pub breaker_cooldown: Duration,
    pub breaker_scope: BreakerScope,
    pub pool_idle_timeout: Duration,
    pub pool_max_idle_per_host: usize,
    /// Whether the target's own 5xx counts as a failure (retried, counted
    /// for the breaker) or is passed on as an answer.
    pub upstream_5xx_is_failure: bool,
}

impl UpstreamConfig {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            attempt_timeout: Duration::from_millis(2500),
            connect_timeout: Duration::from_millis(1000),
            budget: None,
            retry_attempts: 2,
            retry_base_delay: Duration::from_millis(200),
            retry_max_delay: Duration::from_millis(2000),
            breaker_threshold: 5,
            breaker_cooldown: Duration::from_secs(30),
            breaker_scope: BreakerScope::Target,
            pool_idle_timeout: Duration::from_millis(90_000),
            pool_max_idle_per_host: 32,
            upstream_5xx_is_failure: false,
        }
    }
}

/// How [`UpstreamClient::await_ready`] waits.
#[derive(Debug, Clone, Copy)]
pub struct ReadyBackoff {
    pub first_delay: Duration,
    pub max_delay: Duration,
    /// The whole wait.
    pub deadline: Duration,
}

impl Default for ReadyBackoff {
    /// 250 ms, doubling to 5 s, for at most two minutes.
    fn default() -> Self {
        Self {
            first_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(5),
            deadline: Duration::from_secs(120),
        }
    }
}

/// What one breaker covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakerScope {
    /// One breaker per `service://…` target.
    Target,
    /// One per target and first path segment (`<target> /work`), so a slow
    /// route family cannot lock everyone out of `/auth/me`.
    TargetAndRouteFamily,
}

/// The host's call envelope.
#[derive(Debug, Clone, Serialize)]
pub struct GatewayCall {
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verb: Option<String>,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
    pub headers: HashMap<String, String>,
    pub query: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

impl GatewayCall {
    pub fn new(target: impl Into<String>, method: &str, path: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            verb: Some(method.trim().to_ascii_uppercase()),
            path: path.into(),
            body: None,
            headers: HashMap::new(),
            query: HashMap::new(),
            timeout_ms: None,
        }
    }

    #[must_use]
    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    #[must_use]
    pub fn query(mut self, query: HashMap<String, String>) -> Self {
        self.query = query;
        self
    }

    #[must_use]
    pub fn body(mut self, body: Option<Value>) -> Self {
        self.body = body;
        self
    }

    fn method(&self) -> &str {
        self.verb.as_deref().unwrap_or("POST")
    }
}

/// The target's answer, as the host relays it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayReply {
    pub status: u16,
    pub audit_id: String,
    pub headers: HashMap<String, String>,
    pub body: GatewayBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "format", content = "value")]
pub enum GatewayBody {
    Empty,
    Json(Value),
    Text(String),
    Base64(String),
    RawJson(Box<serde_json::value::RawValue>),
}

impl GatewayBody {
    /// The body as a JSON value (free for `Json`, parsed for `RawJson`).
    pub fn as_json_value(&self) -> Option<Value> {
        match self {
            Self::Json(value) => Some(value.clone()),
            Self::RawJson(raw) => serde_json::from_str(raw.get()).ok(),
            _ => None,
        }
    }

    /// The body as `T`, without an intermediate tree for `RawJson`.
    pub fn deserialize_as<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        match self {
            Self::Json(value) => serde_json::from_value(value.clone()).ok(),
            Self::RawJson(raw) => serde_json::from_str(raw.get()).ok(),
            _ => None,
        }
    }
}

/// The host's answer when it could not make the call.
#[derive(Debug, Deserialize)]
struct HostError {
    status: u16,
    error: String,
}

/// Why a call did not produce an answer from the target.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UpstreamError {
    /// The breaker is open: not dialled.
    #[error("circuit open for {key}")]
    CircuitOpen {
        key: String,
        retry_after_seconds: u64,
    },
    #[error("upstream call timed out")]
    Timeout,
    /// No connection, or the connection broke.
    #[error("transport failure: {0}")]
    Transport(String),
    /// A 2xx from the host that is not its envelope. Never holds the body
    /// (a 2xx from auth-service carries tokens).
    #[error("unreadable host answer: {0}")]
    InvalidResponse(String),
    /// The host refused or failed the call itself (`{status, error}`).
    #[error("host gateway answered {status}: {message}")]
    Gateway { status: u16, message: String },
    #[error("invalid client configuration: {0}")]
    Config(String),
}

impl UpstreamError {
    /// Whether this says the target (or the way to it) is in trouble: the
    /// only failures that are retried and that count for its breaker.
    pub fn is_infrastructure_failure(&self) -> bool {
        match self {
            Self::Timeout | Self::Transport(_) | Self::InvalidResponse(_) => true,
            Self::Gateway { status, .. } => counts_as_failure(*status),
            Self::CircuitOpen { .. } | Self::Config(_) => false,
        }
    }

    pub fn class(&self) -> ErrorClass {
        match self {
            Self::Gateway { status, .. } => ErrorClass::of_status(*status),
            Self::Config(_) => ErrorClass::Internal,
            Self::CircuitOpen { .. }
            | Self::Timeout
            | Self::Transport(_)
            | Self::InvalidResponse(_) => ErrorClass::Unavailable,
        }
    }

    /// The HTTP status a gateway reports this failure with: 504 for a
    /// timeout, 502 for a broken or unreadable way, 503 for an open
    /// breaker, the host's own status otherwise.
    pub fn status(&self) -> u16 {
        match self {
            Self::CircuitOpen { .. } => 503,
            Self::Timeout => 504,
            Self::Transport(_) | Self::InvalidResponse(_) => 502,
            Self::Gateway { status, .. } => *status,
            Self::Config(_) => 500,
        }
    }
}

/// Timeouts, transport failures and 5xx (408 included) open a breaker.
pub fn counts_as_failure(status: u16) -> bool {
    status == 408 || status >= 500
}

/// GET, HEAD, OPTIONS, PUT and DELETE; never POST or PATCH.
pub fn is_idempotent(method: &str) -> bool {
    matches!(
        method.trim().to_ascii_uppercase().as_str(),
        "GET" | "HEAD" | "OPTIONS" | "PUT" | "DELETE"
    )
}

/// `base * 2^index`, at most `max` (at least `base`).
pub fn retry_delay(base: Duration, max: Duration, retry_index: u32) -> Duration {
    if base.is_zero() {
        return Duration::ZERO;
    }
    let multiplier = 2u32.saturating_pow(retry_index);
    base.saturating_mul(multiplier).min(max.max(base))
}

/// The first path segment (`/work/tickets/x` → `work`).
pub fn route_family(path: &str) -> &str {
    path.trim_start_matches('/')
        .split(['/', '?'])
        .next()
        .unwrap_or_default()
}

/// The headers of `headers` named in `allowlist`, as the host envelope
/// carries them (values that are not text, or blank, are dropped).
pub fn forward_allowlisted(headers: &HeaderMap, allowlist: &[&str]) -> HashMap<String, String> {
    let mut forwarded = HashMap::with_capacity(allowlist.len() + 2);
    for name in allowlist {
        if let Some(value) = headers.get(*name).and_then(|value| value.to_str().ok()) {
            if !value.trim().is_empty() {
                forwarded.insert((*name).to_string(), value.to_string());
            }
        }
    }
    forwarded
}

/// Hooks for metrics; every method defaults to doing nothing.
pub trait UpstreamObserver: Send + Sync {
    fn on_retry(&self, _breaker: &str) {}
    fn on_timeout(&self) {}
    fn on_circuit_blocked(&self, _breaker: &str) {}
    fn on_circuit_opened(&self, _breaker: &str) {}
}

struct NoObserver;
impl UpstreamObserver for NoObserver {}

#[derive(Debug, Default)]
struct Breaker {
    failures: u32,
    opened_until: Option<Instant>,
    /// Half-open with the probe in flight since then. A probe whose
    /// request was dropped never reports back, so after the probe window
    /// another one is let through instead of the breaker staying shut.
    probing: Option<Instant>,
}

#[derive(Debug, PartialEq, Eq)]
enum Admission {
    Allowed,
    /// Open; the seconds until a probe will be let through.
    Refused {
        retry_after_seconds: u64,
    },
}

pub struct UpstreamClient {
    http: reqwest::Client,
    endpoint: reqwest::Url,
    config: UpstreamConfig,
    breakers: Mutex<HashMap<String, Breaker>>,
    observer: Arc<dyn UpstreamObserver>,
}

impl std::fmt::Debug for UpstreamClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpstreamClient")
            .field("endpoint", &self.endpoint.as_str())
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl UpstreamClient {
    pub fn new(config: UpstreamConfig) -> Result<Self, UpstreamError> {
        let endpoint = reqwest::Url::parse(config.endpoint.trim())
            .map_err(|err| UpstreamError::Config(format!("endpoint: {err}")))?;
        // The host gateway is on loopback: no proxy, no redirects.
        let mut builder = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .pool_max_idle_per_host(config.pool_max_idle_per_host.max(1));
        if !config.connect_timeout.is_zero() {
            builder = builder.connect_timeout(config.connect_timeout);
        }
        if !config.pool_idle_timeout.is_zero() {
            builder = builder.pool_idle_timeout(config.pool_idle_timeout);
        }
        let http = builder
            .build()
            .map_err(|err| UpstreamError::Config(err.to_string()))?;
        Ok(Self {
            http,
            endpoint,
            config,
            breakers: Mutex::new(HashMap::new()),
            observer: Arc::new(NoObserver),
        })
    }

    #[must_use]
    pub fn with_observer(mut self, observer: Arc<dyn UpstreamObserver>) -> Self {
        self.observer = observer;
        self
    }

    pub fn config(&self) -> &UpstreamConfig {
        &self.config
    }

    /// One call through the host, with retries, budget and breaker.
    pub async fn call(&self, mut call: GatewayCall) -> Result<GatewayReply, UpstreamError> {
        let key = self.breaker_key(&call);
        if let Admission::Refused {
            retry_after_seconds,
        } = self.admit(&key)
        {
            self.observer.on_circuit_blocked(&key);
            tracing::warn!(event = "upstream.circuit_open", breaker = %key, "failing fast");
            return Err(UpstreamError::CircuitOpen {
                key,
                retry_after_seconds,
            });
        }

        let max_attempts = if is_idempotent(call.method()) {
            self.config.retry_attempts.saturating_add(1)
        } else {
            1
        };
        let started = Instant::now();
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let remaining = self
                .config
                .budget
                .map(|budget| budget.saturating_sub(started.elapsed()));
            if remaining.is_some_and(|left| left.is_zero()) {
                self.observer.on_timeout();
                return Err(UpstreamError::Timeout);
            }
            let timeout = attempt_timeout(self.config.attempt_timeout, remaining);
            call.timeout_ms =
                timeout.map(|limit| u64::try_from(limit.as_millis()).unwrap_or(u64::MAX));

            let failure = match self.call_once(&call, timeout).await {
                Ok(reply) if !(self.config.upstream_5xx_is_failure && reply.status >= 500) => {
                    self.record_reached(&key);
                    return Ok(reply);
                }
                Ok(reply) => Ok(reply),
                Err(err) if !err.is_infrastructure_failure() => {
                    // The target (or the host) answered: it is up.
                    self.record_reached(&key);
                    return Err(err);
                }
                Err(err) => {
                    if err == UpstreamError::Timeout {
                        self.observer.on_timeout();
                    }
                    Err(err)
                }
            };

            let opened = self.record_failure(&key);
            if opened {
                self.observer.on_circuit_opened(&key);
                tracing::warn!(
                    event = "upstream.circuit_opened",
                    breaker = %key,
                    cooldown_seconds = self.config.breaker_cooldown.as_secs(),
                );
            }
            let delay = retry_delay(
                self.config.retry_base_delay,
                self.config.retry_max_delay,
                attempt - 1,
            );
            let out_of_budget = self
                .config
                .budget
                .is_some_and(|budget| started.elapsed().saturating_add(delay) >= budget);
            if opened || attempt >= max_attempts || out_of_budget {
                return failure;
            }
            self.observer.on_retry(&key);
            tracing::warn!(
                event = "upstream.retry",
                breaker = %key,
                attempt,
                max_attempts,
                status = failure.as_ref().map_or_else(UpstreamError::status, |reply| reply.status),
                delay_ms = u64::try_from(delay.as_millis()).unwrap_or(u64::MAX),
            );
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
        }
    }

    /// Calls `call` until the target answers — anything but an
    /// infrastructure failure — waiting with exponential back-off in
    /// between, at most `backoff.deadline` in all.
    ///
    /// For startup checks that run while the target may still be starting:
    /// the attempts do not count for the target's breaker, and waiting is
    /// logged at debug level only; a warning comes only if the deadline
    /// passes. The last outcome is returned then.
    pub async fn await_ready(
        &self,
        call: GatewayCall,
        backoff: ReadyBackoff,
    ) -> Result<GatewayReply, UpstreamError> {
        let started = Instant::now();
        let mut delay = backoff.first_delay;
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let mut call = call.clone();
            let timeout = attempt_timeout(self.config.attempt_timeout, None);
            call.timeout_ms =
                timeout.map(|limit| u64::try_from(limit.as_millis()).unwrap_or(u64::MAX));
            let outcome = self.call_once(&call, timeout).await;
            let ready = match &outcome {
                Ok(reply) => !(self.config.upstream_5xx_is_failure && reply.status >= 500),
                Err(err) => !err.is_infrastructure_failure(),
            };
            if ready {
                if attempt > 1 {
                    tracing::info!(
                        event = "upstream.ready",
                        target = %call.target,
                        attempts = attempt,
                        waited_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                    );
                }
                return outcome;
            }
            if started.elapsed().saturating_add(delay) >= backoff.deadline {
                tracing::warn!(
                    event = "upstream.not_ready",
                    target = %call.target,
                    attempts = attempt,
                    error = %outcome.as_ref().map_or_else(ToString::to_string, |reply| format!("status {}", reply.status)),
                    "gave up waiting for the upstream to answer"
                );
                return outcome;
            }
            tracing::debug!(
                event = "upstream.waiting",
                target = %call.target,
                attempt,
                delay_ms = u64::try_from(delay.as_millis()).unwrap_or(u64::MAX),
            );
            tokio::time::sleep(delay).await;
            delay = delay
                .saturating_mul(2)
                .min(backoff.max_delay.max(backoff.first_delay));
        }
    }

    async fn call_once(
        &self,
        call: &GatewayCall,
        timeout: Option<Duration>,
    ) -> Result<GatewayReply, UpstreamError> {
        let mut request = self.http.post(self.endpoint.clone()).json(call);
        if let Some(limit) = timeout {
            request = request.timeout(limit);
        }
        let response = request.send().await.map_err(transport_error)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(transport_error)?;
        parse_reply(status, &bytes)
    }

    fn breaker_key(&self, call: &GatewayCall) -> String {
        match self.config.breaker_scope {
            BreakerScope::Target => call.target.clone(),
            BreakerScope::TargetAndRouteFamily => {
                format!("{} /{}", call.target, route_family(&call.path))
            }
        }
    }

    fn breakers(&self) -> std::sync::MutexGuard<'_, HashMap<String, Breaker>> {
        match self.breakers.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// How long a half-open probe may be out before another is allowed:
    /// one attempt's timeout, at least a second.
    fn probe_window(&self) -> Duration {
        self.config.attempt_timeout.max(Duration::from_secs(1))
    }

    fn admit(&self, key: &str) -> Admission {
        if self.config.breaker_threshold == 0 {
            return Admission::Allowed;
        }
        let probe_window = self.probe_window();
        let mut breakers = self.breakers();
        let Some(state) = breakers.get_mut(key) else {
            return Admission::Allowed;
        };
        let Some(until) = state.opened_until else {
            return Admission::Allowed;
        };
        let now = Instant::now();
        if now < until {
            return Admission::Refused {
                retry_after_seconds: until.duration_since(now).as_secs().max(1),
            };
        }
        if let Some(since) = state.probing {
            if now.duration_since(since) < probe_window {
                // Half-open, and the one probe is still out.
                return Admission::Refused {
                    retry_after_seconds: 1,
                };
            }
        }
        state.probing = Some(now);
        Admission::Allowed
    }

    /// The target answered: close its breaker.
    fn record_reached(&self, key: &str) {
        if self.config.breaker_threshold == 0 {
            return;
        }
        if let Some(state) = self.breakers().get_mut(key) {
            *state = Breaker::default();
        }
    }

    /// An infrastructure failure. Returns whether it opened the breaker.
    fn record_failure(&self, key: &str) -> bool {
        if self.config.breaker_threshold == 0 {
            return false;
        }
        let threshold = self.config.breaker_threshold;
        let cooldown = self.config.breaker_cooldown;
        let mut breakers = self.breakers();
        let state = breakers.entry(key.to_string()).or_default();
        let open = if state.probing.is_some() {
            // The probe failed: straight back to open.
            true
        } else {
            state.failures = state.failures.saturating_add(1);
            state.failures >= threshold
        };
        if open {
            *state = Breaker {
                failures: 0,
                opened_until: Some(Instant::now() + cooldown),
                probing: None,
            };
        }
        open
    }
}

/// This attempt's timeout: `attempt` (when set), at most what is left of
/// the budget.
fn attempt_timeout(attempt: Duration, remaining: Option<Duration>) -> Option<Duration> {
    let attempt = (!attempt.is_zero()).then_some(attempt);
    match (attempt, remaining) {
        (Some(attempt), Some(left)) => Some(attempt.min(left)),
        (Some(limit), None) | (None, Some(limit)) => Some(limit),
        (None, None) => None,
    }
}

fn transport_error(err: reqwest::Error) -> UpstreamError {
    if err.is_timeout() {
        UpstreamError::Timeout
    } else {
        UpstreamError::Transport(err.to_string())
    }
}

fn parse_reply(status: StatusCode, bytes: &[u8]) -> Result<GatewayReply, UpstreamError> {
    if status.is_success() {
        return serde_json::from_slice(bytes).map_err(|err| {
            UpstreamError::InvalidResponse(format!("{err} ({} bytes)", bytes.len()))
        });
    }
    if let Ok(host) = serde_json::from_slice::<HostError>(bytes) {
        tracing::warn!(
            event = "upstream.host_error",
            status = host.status,
            error = %excerpt(&host.error, 200),
        );
        return Err(UpstreamError::Gateway {
            status: host.status,
            message: host.error,
        });
    }
    let text = excerpt(&String::from_utf8_lossy(bytes), 200);
    tracing::warn!(event = "upstream.host_error", status = status.as_u16(), body = %text);
    Err(UpstreamError::Gateway {
        status: status.as_u16(),
        message: text,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::extract::State;
    use axum::routing::post;
    use axum::{Json, Router};
    use serde_json::json;

    use super::*;

    const CORE: &str = "service://module:athene::core";

    #[derive(Clone)]
    struct Host {
        hits: Arc<Mutex<HashMap<String, usize>>>,
        calls: Arc<AtomicUsize>,
        last_timeout: Arc<Mutex<Option<u64>>>,
        delay: Duration,
    }

    impl Host {
        fn new(delay: Duration) -> Self {
            Self {
                hits: Arc::default(),
                calls: Arc::default(),
                last_timeout: Arc::default(),
                delay,
            }
        }

        fn hits(&self, path: &str) -> usize {
            self.hits.lock().unwrap().get(path).copied().unwrap_or(0)
        }
    }

    /// The host's `/call`: `/down` 503 from the host, `/limited` 429 from
    /// the host, `/broken` a 500 from the target, `/garbage` an unreadable
    /// 2xx, everything else a 200 from the target.
    async fn call_handler(
        State(host): State<Host>,
        Json(call): Json<Value>,
    ) -> (axum::http::StatusCode, String) {
        let path = call["path"].as_str().unwrap_or_default().to_string();
        *host.hits.lock().unwrap().entry(path.clone()).or_default() += 1;
        host.calls.fetch_add(1, Ordering::SeqCst);
        *host.last_timeout.lock().unwrap() = call["timeout_ms"].as_u64();
        if !host.delay.is_zero() {
            tokio::time::sleep(host.delay).await;
        }
        let reply = |status: u16| {
            json!({
                "status": status, "audit_id": "a", "headers": {},
                "body": { "format": "Json", "value": { "data": { "ok": status < 400 } } }
            })
            .to_string()
        };
        match path.as_str() {
            "/down" => (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                json!({ "status": 503, "error": "upstream down" }).to_string(),
            ),
            "/limited" => (
                axum::http::StatusCode::TOO_MANY_REQUESTS,
                json!({ "status": 429, "error": "rate limited" }).to_string(),
            ),
            "/broken" => (axum::http::StatusCode::OK, reply(500)),
            "/denied" => (axum::http::StatusCode::OK, reply(401)),
            "/garbage" => (axum::http::StatusCode::OK, "not the envelope".to_string()),
            _ => (axum::http::StatusCode::OK, reply(200)),
        }
    }

    async fn start(host: Host) -> String {
        let app = Router::new()
            .route("/call", post(call_handler))
            .with_state(host);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}/call")
    }

    fn config(endpoint: String, retries: u32, threshold: u32) -> UpstreamConfig {
        UpstreamConfig {
            retry_attempts: retries,
            retry_base_delay: Duration::from_millis(1),
            retry_max_delay: Duration::from_millis(2),
            breaker_threshold: threshold,
            attempt_timeout: Duration::from_millis(500),
            ..UpstreamConfig::new(endpoint)
        }
    }

    async fn get(client: &UpstreamClient, path: &str) -> Result<u16, UpstreamError> {
        client
            .call(GatewayCall::new(CORE, "get", path))
            .await
            .map(|reply| reply.status)
    }

    #[tokio::test]
    async fn host_failures_are_retried_for_idempotent_methods_only() {
        let host = Host::new(Duration::ZERO);
        let client = UpstreamClient::new(config(start(host.clone()).await, 2, 0)).unwrap();
        assert_eq!(get(&client, "/down").await.unwrap_err().status(), 503);
        assert_eq!(host.hits("/down"), 3, "1 + 2 retries");

        let post = client
            .call(GatewayCall::new(CORE, "POST", "/down"))
            .await
            .unwrap_err();
        assert_eq!(post.status(), 503);
        assert_eq!(host.hits("/down"), 4, "a POST is sent once");
        let patch = client.call(GatewayCall::new(CORE, "PATCH", "/down")).await;
        assert!(patch.is_err());
        assert_eq!(host.hits("/down"), 5, "a PATCH is sent once");
    }

    #[tokio::test]
    async fn a_rate_limit_or_a_4xx_is_neither_retried_nor_counted() {
        let host = Host::new(Duration::ZERO);
        let client = UpstreamClient::new(config(start(host.clone()).await, 2, 2)).unwrap();
        for _ in 0..6 {
            let err = get(&client, "/limited").await.unwrap_err();
            assert_eq!(err.class(), ErrorClass::RateLimited);
        }
        assert_eq!(host.hits("/limited"), 6, "every call reached the host once");
        // A target's 401 is an answer, passed on.
        assert_eq!(get(&client, "/denied").await, Ok(401));
        assert_eq!(get(&client, "/ok").await, Ok(200), "no breaker opened");
    }

    #[tokio::test]
    async fn a_targets_5xx_counts_only_when_asked_to() {
        let host = Host::new(Duration::ZERO);
        let endpoint = start(host.clone()).await;
        let passed_on = UpstreamClient::new(config(endpoint.clone(), 2, 0)).unwrap();
        assert_eq!(get(&passed_on, "/broken").await, Ok(500));
        assert_eq!(host.hits("/broken"), 1);

        let counted = UpstreamClient::new(UpstreamConfig {
            upstream_5xx_is_failure: true,
            ..config(endpoint, 2, 0)
        })
        .unwrap();
        assert_eq!(get(&counted, "/broken").await, Ok(500), "the last answer");
        assert_eq!(host.hits("/broken"), 4, "retried twice");
    }

    #[tokio::test]
    async fn breakers_are_per_target_or_route_family() {
        let host = Host::new(Duration::ZERO);
        let endpoint = start(host.clone()).await;
        let client = UpstreamClient::new(UpstreamConfig {
            breaker_scope: BreakerScope::TargetAndRouteFamily,
            ..config(endpoint.clone(), 0, 2)
        })
        .unwrap();
        // `/down` is its own family.
        assert!(get(&client, "/down").await.is_err());
        assert!(get(&client, "/down").await.is_err());
        let open = get(&client, "/down").await.unwrap_err();
        assert!(
            matches!(open, UpstreamError::CircuitOpen { .. }),
            "{open:?}"
        );
        assert_eq!(open.status(), 503);
        assert_eq!(host.hits("/down"), 2, "not dialled while open");
        assert_eq!(get(&client, "/ok").await, Ok(200), "another family");

        let per_target = UpstreamClient::new(config(endpoint, 0, 2)).unwrap();
        assert!(get(&per_target, "/down").await.is_err());
        assert!(get(&per_target, "/down").await.is_err());
        assert!(matches!(
            get(&per_target, "/ok").await,
            Err(UpstreamError::CircuitOpen { .. })
        ));
        let other = per_target
            .call(GatewayCall::new(
                "service://module:other::svc",
                "GET",
                "/ok",
            ))
            .await;
        assert_eq!(other.map(|r| r.status), Ok(200), "another target");
    }

    #[test]
    fn a_half_open_breaker_lets_one_probe_through() {
        let client = UpstreamClient::new(UpstreamConfig {
            breaker_threshold: 1,
            ..UpstreamConfig::new("http://127.0.0.1:1/call")
        })
        .unwrap();
        let key = "t";
        assert!(client.record_failure(key), "threshold 1 opens");
        assert!(matches!(client.admit(key), Admission::Refused { .. }));

        let expire = |client: &UpstreamClient| {
            let mut breakers = client.breakers();
            let state = breakers.get_mut(key).unwrap();
            state.opened_until = Some(
                Instant::now()
                    .checked_sub(Duration::from_millis(1))
                    .unwrap(),
            );
        };
        expire(&client);
        assert_eq!(client.admit(key), Admission::Allowed, "the probe");
        assert!(
            matches!(client.admit(key), Admission::Refused { .. }),
            "everyone else waits for the probe"
        );

        // A failed probe opens it again for a full cooldown.
        assert!(client.record_failure(key));
        assert!(matches!(
            client.admit(key),
            Admission::Refused { retry_after_seconds } if retry_after_seconds >= 29
        ));

        // A probe that never reports back does not keep it shut.
        expire(&client);
        assert_eq!(client.admit(key), Admission::Allowed);
        {
            let mut breakers = client.breakers();
            breakers.get_mut(key).unwrap().probing =
                Some(Instant::now().checked_sub(client.probe_window()).unwrap());
        }
        assert_eq!(client.admit(key), Admission::Allowed, "a new probe");
        assert!(client.record_failure(key), "which fails again");

        // A probe that reaches the target closes it.
        expire(&client);
        assert_eq!(client.admit(key), Admission::Allowed);
        client.record_reached(key);
        assert_eq!(client.admit(key), Admission::Allowed);
        assert_eq!(client.admit(key), Admission::Allowed);
    }

    #[tokio::test]
    async fn failures_map_onto_classes_and_statuses() {
        let unreachable =
            UpstreamClient::new(config("http://127.0.0.1:1/call".into(), 0, 5)).unwrap();
        let err = get(&unreachable, "/x").await.unwrap_err();
        assert!(matches!(err, UpstreamError::Transport(_)), "{err:?}");
        assert_eq!((err.class(), err.status()), (ErrorClass::Unavailable, 502));

        let host = Host::new(Duration::ZERO);
        let client = UpstreamClient::new(config(start(host).await, 0, 5)).unwrap();
        let err = get(&client, "/garbage").await.unwrap_err();
        assert!(
            matches!(&err, UpstreamError::InvalidResponse(m) if !m.contains("not the envelope"))
        );
        assert_eq!(err.status(), 502);
    }

    #[tokio::test]
    async fn the_budget_caps_the_whole_call() {
        let host = Host::new(Duration::from_millis(150));
        let endpoint = start(host.clone()).await;
        let client = UpstreamClient::new(UpstreamConfig {
            attempt_timeout: Duration::from_millis(100),
            budget: Some(Duration::from_millis(250)),
            retry_attempts: 5,
            ..config(endpoint, 5, 0)
        })
        .unwrap();
        let started = Instant::now();
        let err = get(&client, "/slow").await.unwrap_err();
        assert_eq!(err, UpstreamError::Timeout);
        assert!(
            started.elapsed() < Duration::from_millis(600),
            "{:?}",
            started.elapsed()
        );
        assert!(host.calls.load(Ordering::SeqCst) <= 3);
        // The host is told the per-attempt timeout, never more than is left.
        assert!(host.last_timeout.lock().unwrap().unwrap() <= 100);
    }

    #[tokio::test]
    async fn await_ready_waits_for_a_target_that_is_still_starting() {
        // Nothing listens yet; the host comes up after ~200 ms.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let host = Host::new(Duration::ZERO);
        let late = host.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let app = Router::new()
                .route("/call", post(call_handler))
                .with_state(late);
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, app).await.unwrap();
        });
        let client = UpstreamClient::new(config(format!("http://{addr}/call"), 0, 1)).unwrap();
        let backoff = ReadyBackoff {
            first_delay: Duration::from_millis(20),
            max_delay: Duration::from_millis(80),
            deadline: Duration::from_secs(5),
        };
        let reply = client
            .await_ready(GatewayCall::new(CORE, "GET", "/ok"), backoff)
            .await
            .unwrap();
        assert_eq!(reply.status, 200);
        // The failed attempts did not open the breaker (threshold 1).
        assert_eq!(get(&client, "/ok").await, Ok(200));

        // A deadline that passes returns the last failure.
        let never = UpstreamClient::new(config("http://127.0.0.1:1/call".into(), 0, 1)).unwrap();
        let short = ReadyBackoff {
            deadline: Duration::from_millis(60),
            ..backoff
        };
        assert!(matches!(
            never
                .await_ready(GatewayCall::new(CORE, "GET", "/ok"), short)
                .await,
            Err(UpstreamError::Transport(_))
        ));
    }

    #[test]
    fn helpers() {
        assert_eq!(route_family("/auth/me"), "auth");
        assert_eq!(route_family("/work?limit=1"), "work");
        assert!(is_idempotent("get") && is_idempotent("DELETE") && is_idempotent("PUT"));
        assert!(!is_idempotent("POST") && !is_idempotent("patch"));
        let ms = Duration::from_millis;
        assert_eq!(retry_delay(ms(200), ms(2000), 0), ms(200));
        assert_eq!(retry_delay(ms(200), ms(2000), 3), ms(1600));
        assert_eq!(retry_delay(ms(200), ms(2000), 9), ms(2000));
        assert_eq!(retry_delay(Duration::ZERO, ms(2000), 3), Duration::ZERO);
        assert_eq!(attempt_timeout(ms(100), Some(ms(40))), Some(ms(40)));
        assert_eq!(attempt_timeout(Duration::ZERO, None), None);

        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer x".parse().unwrap());
        headers.insert("x-request-id", "  ".parse().unwrap());
        headers.insert("cookie", "secret".parse().unwrap());
        let forwarded = forward_allowlisted(&headers, &["authorization", "x-request-id"]);
        assert_eq!(forwarded.len(), 1);
        assert_eq!(forwarded["authorization"], "Bearer x");
    }
}
