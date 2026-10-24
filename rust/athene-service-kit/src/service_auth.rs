//! Service-to-service authentication, fail-closed (S2; BE-20, SEC-08).
//!
//! Callers present `x-fenrir-service-token: Bearer <token>`. The receiver
//! compares it, in constant time, with the token it is configured with
//! (`<PREFIX>_INTERNAL_SERVICE_AUTH_TOKEN` or `..._FILE`).
//!
//! **Modes.** `enforce` refuses a request without a valid token (the
//! answer is below). `report` lets it through and logs a structured warning
//! once per caller — a rollout stage only. **Enforce is the default**: the
//! old default, "enforce when a token is set", switched service auth off
//! exactly when the token was missing. Enforcing without a token is a
//! configuration error ([`ServiceAuthConfig::validate`]) and, should a
//! service run like that anyway, every request is refused; report mode must
//! be asked for (`<PREFIX>_ENFORCE_INTERNAL_SERVICE_AUTH=false`).
//!
//! Probe and host-scrape paths stay open ([`PROBE_PATHS`]): the host calls
//! them without a token, and the listener is loopback only.
//!
//! A request with a valid token gets the [`AuthenticatedService`] extension;
//! only such a request may carry a principal the core trusts
//! ([`crate::principal`]).
//!
//! **The refusal is not a 401** (3.6b, S2). A refused service token is
//! `503` [`REFUSED_CODE`] (`ATH-SYS-006`), retryable, with `Retry-After`
//! ([`refused_response`], or the module's own envelope with that code). A
//! 401 means "this user's token is bad" to every hop above: the gateway
//! answers it with a 401 and the web refreshes and signs the user out. A
//! service token that does not match is a deployment fault between two
//! services, the same for every user, so it must read as an outage — the
//! user's session is untouched and comes back once the tokens agree.
//!
//! [`ServiceCredential`] is the other side: the token a caller sends.

use std::collections::HashSet;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use http::{HeaderMap, Request, Response};
use subtle::ConstantTimeEq;
use tower::{Layer, Service};

use crate::client_ip::peer_of;
use crate::config::{flag_from, secret_from, ConfigError, Secret};
use crate::util::{unavailable_response, BoxFuture, Render};

pub const SERVICE_TOKEN_HEADER: &str = "x-fenrir-service-token";
/// Informational only: the host sets it on public ingress; never trusted.
pub const ACTOR_HEADER: &str = "x-fenrir-actor";
/// Shortest token accepted; `openssl rand -hex 32` gives 64 characters.
pub const MIN_TOKEN_LEN: usize = 32;
/// Health, readiness and host scrape endpoints, open without a token.
pub const PROBE_PATHS: &[&str] = &[
    "/health",
    "/live",
    "/ready",
    "/metrics/internal",
    "/metrics/prometheus",
    "/.fenrir/services",
];
/// Upper bound on distinct callers remembered for report-mode logging.
const MAX_REPORTED_CALLERS: usize = 256;
/// The code of a refused service credential: `503`, retryable, never a 401
/// (the module doc says why). Every service answers it with this code; the
/// gateway recognises it whatever the status (athene-api
/// `transport/http/response.rs`).
pub const REFUSED_CODE: &str = "ATH-SYS-006";
/// `Retry-After` of [`refused_response`]: a mismatch is a deployment fault
/// and will not clear within a second, but it does clear when the tokens
/// agree again.
pub const REFUSED_RETRY_AFTER_SECS: u64 = 5;
/// `details.reason` of [`refused_response`]: the same word for every
/// refusal, so a client learns nothing about the token from it.
pub const REFUSED_REASON: &str = "service_auth";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceAuthMode {
    /// Allow every caller, but log a structured warning once per caller.
    Report,
    /// Refuse callers without a valid token.
    Enforce,
}

impl ServiceAuthMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Report => "report",
            Self::Enforce => "enforce",
        }
    }
}

/// The environment variables a service reads its settings from, aliases in
/// order of preference (`<VAR>_FILE` is tried for every token name).
#[derive(Debug, Clone, Copy)]
pub struct ServiceAuthVars<'a> {
    pub token: &'a [&'a str],
    pub enforce: &'a [&'a str],
}

/// Service-auth settings. `Debug` never shows the token.
#[derive(Clone, PartialEq, Eq)]
pub struct ServiceAuthConfig {
    mode: ServiceAuthMode,
    token: Option<Secret>,
    token_var: String,
    enforce_var: String,
    open_paths: Vec<String>,
}

impl fmt::Debug for ServiceAuthConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceAuthConfig")
            .field("mode", &self.mode)
            .field("token", &self.token.as_ref().map(|_| "<redacted>"))
            .field("open_paths", &self.open_paths)
            .finish()
    }
}

impl ServiceAuthConfig {
    pub fn new(mode: ServiceAuthMode, token: Option<Secret>) -> Self {
        Self {
            mode,
            token,
            token_var: "the service token".to_string(),
            enforce_var: "the enforce flag".to_string(),
            open_paths: PROBE_PATHS.iter().map(|path| (*path).to_string()).collect(),
        }
    }

    /// Enforce with `token`.
    pub fn enforce(token: Secret) -> Self {
        Self::new(ServiceAuthMode::Enforce, Some(token))
    }

    /// Report mode, with or without a token.
    pub fn report(token: Option<Secret>) -> Self {
        Self::new(ServiceAuthMode::Report, token)
    }

    /// The settings through `lookup` (the process environment in
    /// production):
    /// - the token from the first of `vars.token` (or its `_FILE`); it must
    ///   be at least [`MIN_TOKEN_LEN`] printable characters;
    /// - `enforce` from the first of `vars.enforce` that is set (strict
    ///   `true`/`false`), **enforce when none is**.
    ///
    /// Enforce without a token is returned as it is; [`Self::validate`]
    /// says that it cannot run.
    pub fn from_lookup(
        lookup: &dyn Fn(&str) -> Option<String>,
        vars: ServiceAuthVars<'_>,
    ) -> Result<Self, ConfigError> {
        let token_var = vars.token.first().copied().unwrap_or("service token");
        let enforce_var = vars.enforce.first().copied().unwrap_or("enforce flag");
        let token = secret_from(lookup, vars.token)?
            .map(|token| validate_token(token_var, token))
            .transpose()?;
        let mode = match flag_from(lookup, vars.enforce)? {
            Some(false) => ServiceAuthMode::Report,
            Some(true) | None => ServiceAuthMode::Enforce,
        };
        Ok(Self {
            token_var: token_var.to_string(),
            enforce_var: enforce_var.to_string(),
            ..Self::new(mode, token)
        })
    }

    /// [`Self::from_lookup`] over the process environment.
    pub fn from_env(vars: ServiceAuthVars<'_>) -> Result<Self, ConfigError> {
        Self::from_lookup(&|name| std::env::var(name).ok(), vars)
    }

    /// Whether the service can run with these settings: enforcing needs a
    /// token. A service refuses to start rather than run without its
    /// boundary (fail-closed).
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.mode == ServiceAuthMode::Enforce && self.token.is_none() {
            return Err(ConfigError::invalid(
                &self.token_var,
                format!(
                    "service auth enforces by default and needs a token: set {0} or {0}_FILE, \
                     or {1}=false for report mode during a rollout",
                    self.token_var, self.enforce_var
                ),
            ));
        }
        Ok(())
    }

    /// `self`, if [`Self::validate`] passes.
    pub fn validated(self) -> Result<Self, ConfigError> {
        self.validate().map(|()| self)
    }

    /// Paths open without a token (default [`PROBE_PATHS`]).
    #[must_use]
    pub fn with_open_paths(mut self, paths: &[&str]) -> Self {
        self.open_paths = paths.iter().map(|path| (*path).to_string()).collect();
        self
    }

    pub fn mode(&self) -> ServiceAuthMode {
        self.mode
    }

    pub fn token(&self) -> Option<&Secret> {
        self.token.as_ref()
    }

    pub fn token_configured(&self) -> bool {
        self.token.is_some()
    }

    pub fn open_paths(&self) -> &[String] {
        &self.open_paths
    }
}

fn validate_token(var: &str, token: Secret) -> Result<Secret, ConfigError> {
    let value = token.expose();
    if value.len() < MIN_TOKEN_LEN {
        return Err(ConfigError::invalid(
            var,
            format!("must be at least {MIN_TOKEN_LEN} characters"),
        ));
    }
    if !value.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(ConfigError::invalid(
            var,
            "must be printable ASCII without spaces",
        ));
    }
    Ok(token)
}

/// What a presented token amounts to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCheck {
    Valid,
    Missing,
    Invalid,
    /// No token configured on this side, so nothing can be valid.
    NotConfigured,
}

impl TokenCheck {
    pub fn reason(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::Missing => "missing_service_token",
            Self::Invalid => "invalid_service_token",
            Self::NotConfigured => "service_token_not_configured",
        }
    }
}

/// Request extension: the caller presented a valid service token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthenticatedService;

/// The check, shared by the layer and handlers that need it themselves.
pub struct ServiceAuth {
    config: ServiceAuthConfig,
    reported: Mutex<HashSet<String>>,
}

impl ServiceAuth {
    pub fn new(config: ServiceAuthConfig) -> Self {
        Self {
            config,
            reported: Mutex::new(HashSet::new()),
        }
    }

    pub fn config(&self) -> &ServiceAuthConfig {
        &self.config
    }

    pub fn mode(&self) -> ServiceAuthMode {
        self.config.mode
    }

    pub fn token_configured(&self) -> bool {
        self.config.token_configured()
    }

    /// The presented token against the configured one, in constant time.
    pub fn check(&self, headers: &HeaderMap) -> TokenCheck {
        let Some(expected) = self.config.token.as_ref() else {
            return TokenCheck::NotConfigured;
        };
        let Some(raw) = headers.get(SERVICE_TOKEN_HEADER) else {
            return TokenCheck::Missing;
        };
        let Ok(value) = raw.to_str() else {
            return TokenCheck::Invalid;
        };
        let Some(presented) = value.trim().strip_prefix("Bearer ") else {
            return TokenCheck::Invalid;
        };
        if constant_time_eq(presented.trim().as_bytes(), expected.expose().as_bytes()) {
            TokenCheck::Valid
        } else {
            TokenCheck::Invalid
        }
    }

    /// Whether a handler may treat the request as a trusted service call:
    /// a valid token; in report mode without a configured token, any
    /// presented token (what such a service accepted before 3.6, logged).
    pub fn trusted(&self, headers: &HeaderMap) -> bool {
        match self.check(headers) {
            TokenCheck::Valid => true,
            TokenCheck::NotConfigured
                if self.mode() == ServiceAuthMode::Report
                    && headers.contains_key(SERVICE_TOKEN_HEADER) =>
            {
                if self.first_report("trusted-call") {
                    tracing::warn!(
                        event = "security.service_auth.report",
                        mode = "report",
                        reason = TokenCheck::NotConfigured.reason(),
                        "service call trusted without a token to check it against (report mode)"
                    );
                }
                true
            }
            _ => false,
        }
    }

    pub fn is_open_path(&self, path: &str) -> bool {
        self.config.open_paths.iter().any(|open| open == path)
    }

    /// The startup line every service writes (S2 deploy check):
    /// `security.service_auth.config mode=… token_configured=…`.
    pub fn log_startup(&self, service: &str) {
        tracing::info!(
            event = "security.service_auth.config",
            service,
            mode = self.mode().as_str(),
            token_configured = self.token_configured(),
            "service authentication configured"
        );
        if self.mode() == ServiceAuthMode::Report {
            tracing::warn!(
                event = "security.service_auth.report_mode",
                service,
                "service auth is in report mode: unauthenticated service calls are allowed"
            );
        }
    }

    /// True the first time `caller` is seen (bounded set).
    fn first_report(&self, caller: &str) -> bool {
        let mut reported = match self.reported.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if reported.contains(caller) || reported.len() >= MAX_REPORTED_CALLERS {
            return false;
        }
        reported.insert(caller.to_string())
    }
}

/// Comparison whose duration does not depend on where the inputs differ.
/// A length mismatch returns early: the length is fixed by configuration.
pub fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    left.ct_eq(right).into()
}

/// What the layer refused.
#[derive(Debug, Clone)]
pub struct ServiceAuthDenied {
    pub check: TokenCheck,
    pub method: String,
    pub path: String,
}

/// The kit's answer to a refused service credential: `503 ATH-SYS-006`
/// in athene's envelope (`message_key: errors.unavailable`, `retryable`,
/// `details {reason: "service_auth", retry_after_seconds}`) with
/// `Retry-After` and the request id. Which check failed (missing, invalid,
/// not configured) goes to the log, not to the caller.
pub fn refused_response<B: From<String>>(_: &ServiceAuthDenied) -> Response<B> {
    unavailable_response(
        REFUSED_CODE,
        "Service authentication refused",
        REFUSED_REASON,
        REFUSED_RETRY_AFTER_SECS,
    )
}

/// Checks the service token of every request but the open paths.
pub struct ServiceAuthLayer<B> {
    auth: Arc<ServiceAuth>,
    reject: Render<ServiceAuthDenied, B>,
}

impl<B> Clone for ServiceAuthLayer<B> {
    fn clone(&self) -> Self {
        Self {
            auth: Arc::clone(&self.auth),
            reject: Arc::clone(&self.reject),
        }
    }
}

impl<B> ServiceAuthLayer<B> {
    /// `reject` answers a refusal; it must keep [`REFUSED_CODE`] and a 503
    /// (see the module doc), as [`refused_response`] does.
    pub fn new(auth: Arc<ServiceAuth>, reject: Render<ServiceAuthDenied, B>) -> Self {
        Self { auth, reject }
    }
}

impl<B: From<String> + 'static> ServiceAuthLayer<B> {
    /// With [`refused_response`].
    pub fn with_default_answer(auth: Arc<ServiceAuth>) -> Self {
        Self::new(auth, Arc::new(refused_response::<B>))
    }
}

impl<S, B> Layer<S> for ServiceAuthLayer<B> {
    type Service = ServiceAuthService<S, B>;

    fn layer(&self, inner: S) -> Self::Service {
        ServiceAuthService {
            inner,
            layer: self.clone(),
        }
    }
}

pub struct ServiceAuthService<S, B> {
    inner: S,
    layer: ServiceAuthLayer<B>,
}

impl<S: Clone, B> Clone for ServiceAuthService<S, B> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            layer: self.layer.clone(),
        }
    }
}

impl<S, ReqB, ResB> Service<Request<ReqB>> for ServiceAuthService<S, ResB>
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

    fn call(&mut self, mut request: Request<ReqB>) -> Self::Future {
        let auth = Arc::clone(&self.layer.auth);
        let path = request.uri().path().to_string();
        if !auth.is_open_path(&path) {
            let check = auth.check(request.headers());
            if check == TokenCheck::Valid {
                request.extensions_mut().insert(AuthenticatedService);
            } else {
                let caller = caller_label(&request);
                let method = request.method().to_string();
                match auth.mode() {
                    ServiceAuthMode::Enforce => {
                        tracing::warn!(
                            event = "security.service_auth.denied",
                            reason = check.reason(),
                            caller = %caller,
                            method = %method,
                            path = %crate::observability::redact::path(&path),
                        );
                        let denied = ServiceAuthDenied {
                            check,
                            method,
                            path,
                        };
                        return Box::pin(std::future::ready(Ok((self.layer.reject)(&denied))));
                    }
                    ServiceAuthMode::Report => {
                        if auth.first_report(&caller) {
                            tracing::warn!(
                                event = "security.service_auth.report",
                                mode = "report",
                                reason = check.reason(),
                                caller = %caller,
                                method = %method,
                                path = %crate::observability::redact::path(&path),
                                "unauthenticated service call allowed (report mode); logged once per caller"
                            );
                        }
                    }
                }
            }
        }
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(async move { inner.call(request).await })
    }
}

/// Caller identity for report-mode dedup: peer IP plus the (untrusted,
/// sanitised) actor header. Never contains token material.
fn caller_label<B>(request: &Request<B>) -> String {
    let peer = peer_of(request.extensions())
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let actor = request
        .headers()
        .get(ACTOR_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(|raw| {
            raw.chars()
                .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'))
                .take(64)
                .collect::<String>()
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "-".to_string());
    format!("{peer}/{actor}")
}

/// Where a caller's host-issued module token comes from (module-kit's
/// `ServiceTokenProvider::current_token`).
pub type TokenSource = Arc<dyn Fn() -> Result<String, String> + Send + Sync>;

/// The token a service sends on its own calls.
///
/// The **configured** token comes first: it is the one a receiver in
/// enforce mode can check. The host-issued module token is the fallback
/// (receivers cannot verify it until the host offers introspection,
/// SEC-08); without either, nothing is sent and the host injects the
/// module token itself.
#[derive(Clone, Default)]
pub struct ServiceCredential {
    configured: Option<Secret>,
    provider: Option<TokenSource>,
}

impl fmt::Debug for ServiceCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceCredential")
            .field(
                "configured",
                &self.configured.as_ref().map(|_| "<redacted>"),
            )
            .field("provider", &self.provider.is_some())
            .finish()
    }
}

impl ServiceCredential {
    pub fn new(configured: Option<Secret>, provider: Option<TokenSource>) -> Self {
        Self {
            configured,
            provider,
        }
    }

    pub fn current(&self) -> Option<String> {
        if let Some(token) = self
            .configured
            .as_ref()
            .map(|secret| secret.expose().trim())
            .filter(|token| !token.is_empty())
        {
            return Some(token.to_string());
        }
        let provider = self.provider.as_ref()?;
        match provider() {
            Ok(token) if !token.trim().is_empty() => Some(token.trim().to_string()),
            Ok(_) => None,
            Err(err) => {
                tracing::warn!(
                    event = "service_auth.token_refresh_failed",
                    error = %err,
                    "could not refresh the host-issued service token"
                );
                None
            }
        }
    }

    /// `Bearer <token>` for [`SERVICE_TOKEN_HEADER`].
    pub fn header_value(&self) -> Option<String> {
        self.current().map(|token| format!("Bearer {token}"))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::convert::Infallible;

    use super::*;
    use http::HeaderValue;
    use tower::ServiceExt;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";
    const VARS: ServiceAuthVars<'static> = ServiceAuthVars {
        token: &["SVC_TOKEN", "LEGACY_TOKEN"],
        enforce: &["SVC_ENFORCE"],
    };

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name| map.get(name).cloned()
    }

    fn headers(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(SERVICE_TOKEN_HEADER, HeaderValue::from_str(value).unwrap());
        headers
    }

    fn enforce() -> ServiceAuth {
        ServiceAuth::new(ServiceAuthConfig::enforce(Secret::new(TOKEN)))
    }

    #[test]
    fn enforce_is_the_default_and_needs_a_token() {
        let config = ServiceAuthConfig::from_lookup(&env(&[]), VARS).unwrap();
        assert_eq!(config.mode(), ServiceAuthMode::Enforce);
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("SVC_TOKEN"), "{err}");
        assert!(err.to_string().contains("SVC_ENFORCE=false"), "{err}");

        let config = ServiceAuthConfig::from_lookup(&env(&[("SVC_TOKEN", TOKEN)]), VARS).unwrap();
        assert_eq!(config.mode(), ServiceAuthMode::Enforce);
        assert!(config.validate().is_ok());

        // An alias counts.
        let config =
            ServiceAuthConfig::from_lookup(&env(&[("LEGACY_TOKEN", TOKEN)]), VARS).unwrap();
        assert!(config.token_configured());
    }

    #[test]
    fn report_mode_must_be_asked_for() {
        let config =
            ServiceAuthConfig::from_lookup(&env(&[("SVC_ENFORCE", "false")]), VARS).unwrap();
        assert_eq!(config.mode(), ServiceAuthMode::Report);
        assert!(config.validate().is_ok());
        assert!(
            ServiceAuthConfig::from_lookup(&env(&[("SVC_ENFORCE", "sometimes")]), VARS).is_err()
        );
    }

    #[test]
    fn weak_tokens_are_refused() {
        for bad in ["short", "has a space in it but is long enough!!"] {
            let err =
                ServiceAuthConfig::from_lookup(&env(&[("SVC_TOKEN", bad)]), VARS).unwrap_err();
            assert!(!err.to_string().contains(bad), "{err}");
        }
    }

    #[test]
    fn config_debug_hides_the_token() {
        let config = ServiceAuthConfig::enforce(Secret::new(TOKEN));
        assert!(!format!("{config:?}").contains(TOKEN));
    }

    #[test]
    fn token_check_outcomes() {
        let auth = enforce();
        assert_eq!(
            auth.check(&headers(&format!("Bearer {TOKEN}"))),
            TokenCheck::Valid
        );
        assert_eq!(auth.check(&HeaderMap::new()), TokenCheck::Missing);
        assert_eq!(auth.check(&headers(TOKEN)), TokenCheck::Invalid);
        assert_eq!(
            auth.check(&headers("Bearer 0123456789abcdef0123456789abcdeX")),
            TokenCheck::Invalid
        );
        assert_eq!(auth.check(&headers("Bearer short")), TokenCheck::Invalid);

        let report = ServiceAuth::new(ServiceAuthConfig::report(None));
        assert_eq!(
            report.check(&headers(&format!("Bearer {TOKEN}"))),
            TokenCheck::NotConfigured
        );
    }

    #[test]
    fn trusted_calls() {
        let auth = enforce();
        assert!(auth.trusted(&headers(&format!("Bearer {TOKEN}"))));
        assert!(!auth.trusted(&headers("Bearer wrong-token")));
        assert!(!auth.trusted(&HeaderMap::new()));

        // Report without a token keeps what such a service accepted.
        let report = ServiceAuth::new(ServiceAuthConfig::report(None));
        assert!(report.trusted(&headers("Bearer anything")));
        assert!(!report.trusted(&HeaderMap::new()));
        // Report with a token: only the token.
        let staged = ServiceAuth::new(ServiceAuthConfig::report(Some(Secret::new(TOKEN))));
        assert!(!staged.trusted(&headers("Bearer anything")));
        assert!(staged.trusted(&headers(&format!("Bearer {TOKEN}"))));

        // Enforce without a token trusts nothing (fail-closed).
        let broken = ServiceAuth::new(ServiceAuthConfig::new(ServiceAuthMode::Enforce, None));
        assert!(!broken.trusted(&headers("Bearer anything")));
    }

    #[test]
    fn report_dedup_is_once_per_caller_and_bounded() {
        let auth = ServiceAuth::new(ServiceAuthConfig::report(None));
        assert!(auth.first_report("127.0.0.1/-"));
        assert!(!auth.first_report("127.0.0.1/-"));
        assert!(auth.first_report("127.0.0.1/auth-service"));
        for index in 0..(MAX_REPORTED_CALLERS * 2) {
            auth.first_report(&format!("caller-{index}"));
        }
        assert_eq!(auth.reported.lock().unwrap().len(), MAX_REPORTED_CALLERS);
    }

    #[test]
    fn constant_time_eq_matches_plain_eq() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }

    fn app(
        config: ServiceAuthConfig,
    ) -> impl Service<Request<()>, Response = Response<String>, Error = Infallible> + Clone {
        let reject: Render<ServiceAuthDenied, String> = Arc::new(|denied: &ServiceAuthDenied| {
            let mut response = Response::new(denied.check.reason().to_string());
            *response.status_mut() = http::StatusCode::UNAUTHORIZED;
            response
        });
        ServiceAuthLayer::new(Arc::new(ServiceAuth::new(config)), reject).layer(tower::service_fn(
            |request: Request<()>| {
                let verified = request.extensions().get::<AuthenticatedService>().is_some();
                std::future::ready(Ok(Response::new(format!("verified={verified}"))))
            },
        ))
    }

    async fn call(
        service: &(impl Service<Request<()>, Response = Response<String>, Error = Infallible> + Clone),
        path: &str,
        token: Option<&str>,
    ) -> (u16, String) {
        let mut request = Request::builder().uri(path).body(()).unwrap();
        if let Some(token) = token {
            request.headers_mut().insert(
                SERVICE_TOKEN_HEADER,
                HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
            );
        }
        let response = service.clone().oneshot(request).await.unwrap();
        (response.status().as_u16(), response.into_body())
    }

    #[tokio::test]
    async fn the_layer_enforces_and_leaves_probes_open() {
        let service = app(ServiceAuthConfig::enforce(Secret::new(TOKEN)));
        assert_eq!(
            call(&service, "/api/thing", None).await,
            (401, "missing_service_token".to_string())
        );
        assert_eq!(
            call(&service, "/api/thing", Some("wrong-token")).await,
            (401, "invalid_service_token".to_string())
        );
        assert_eq!(
            call(&service, "/api/thing", Some(TOKEN)).await,
            (200, "verified=true".to_string())
        );
        for probe in PROBE_PATHS {
            assert_eq!(call(&service, probe, None).await.0, 200, "{probe}");
        }
        // `/health/..` is not `/health`.
        assert_eq!(call(&service, "/health/../api", None).await.0, 401);

        // Enforce without a token refuses everything but the probes.
        let broken = app(ServiceAuthConfig::new(ServiceAuthMode::Enforce, None));
        assert_eq!(
            call(&broken, "/api/thing", Some(TOKEN)).await,
            (401, "service_token_not_configured".to_string())
        );
        assert_eq!(call(&broken, "/health", None).await.0, 200);

        let narrow =
            app(ServiceAuthConfig::enforce(Secret::new(TOKEN)).with_open_paths(&["/health"]));
        assert_eq!(call(&narrow, "/metrics/internal", None).await.0, 401);
    }

    /// S2 (3.6b): a refused service token reads as an outage, never as a
    /// rejected user — one code for every check that failed, a 503 the
    /// caller may retry, and nothing about the token.
    #[tokio::test]
    async fn the_default_refusal_is_a_retryable_503_not_a_401() {
        fn ok(_: Request<()>) -> std::future::Ready<Result<Response<String>, Infallible>> {
            std::future::ready(Ok(Response::new("ok".to_string())))
        }
        let with_default = |config: ServiceAuthConfig| {
            ServiceAuthLayer::<String>::with_default_answer(Arc::new(ServiceAuth::new(config)))
                .layer(tower::service_fn(ok))
        };
        let enforce = with_default(ServiceAuthConfig::enforce(Secret::new(TOKEN)));
        let broken = with_default(ServiceAuthConfig::new(ServiceAuthMode::Enforce, None));
        for (service, token) in [
            (&enforce, None),
            (&enforce, Some("wrong-token")),
            (&broken, Some(TOKEN)),
        ] {
            let mut request = Request::builder().uri("/api/thing").body(()).unwrap();
            if let Some(token) = token {
                request.headers_mut().insert(
                    SERVICE_TOKEN_HEADER,
                    HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
                );
            }
            let response = service.clone().oneshot(request).await.unwrap();
            assert_eq!(
                response.status(),
                http::StatusCode::SERVICE_UNAVAILABLE,
                "{token:?}"
            );
            assert_eq!(response.headers()[http::header::RETRY_AFTER], "5");
            let request_id = response.headers()["x-request-id"].clone();
            let body: serde_json::Value = serde_json::from_str(response.body()).unwrap();
            assert_eq!(body["error"]["code"], REFUSED_CODE);
            assert_eq!(body["error"]["message_key"], "errors.unavailable");
            assert_eq!(body["error"]["retryable"], true);
            assert_eq!(
                body["error"]["details"],
                serde_json::json!({ "reason": "service_auth", "retry_after_seconds": 5 })
            );
            assert_eq!(body["error"]["request_id"], request_id.to_str().unwrap());
            assert!(!response.body().contains(TOKEN));
            assert!(!response.body().contains("missing_service_token"));
        }
        let probe = enforce
            .clone()
            .oneshot(Request::builder().uri("/health").body(()).unwrap())
            .await
            .unwrap();
        assert_eq!(probe.status(), http::StatusCode::OK);
    }

    #[tokio::test]
    async fn report_mode_lets_everyone_through_but_verifies_only_the_token() {
        let service = app(ServiceAuthConfig::report(Some(Secret::new(TOKEN))));
        assert_eq!(
            call(&service, "/api/thing", None).await,
            (200, "verified=false".to_string())
        );
        assert_eq!(
            call(&service, "/api/thing", Some(TOKEN)).await,
            (200, "verified=true".to_string())
        );
    }

    #[test]
    fn the_configured_credential_comes_first() {
        let host: TokenSource = Arc::new(|| Ok("host-issued".to_string()));
        let both = ServiceCredential::new(Some(Secret::new(TOKEN)), Some(Arc::clone(&host)));
        assert_eq!(both.current().as_deref(), Some(TOKEN));
        assert_eq!(both.header_value(), Some(format!("Bearer {TOKEN}")));
        let host_only = ServiceCredential::new(None, Some(host));
        assert_eq!(host_only.current().as_deref(), Some("host-issued"));
        let failing: TokenSource = Arc::new(|| Err("control plane down".to_string()));
        assert_eq!(ServiceCredential::new(None, Some(failing)).current(), None);
        assert_eq!(ServiceCredential::default().current(), None);
        assert!(!format!("{both:?}").contains(TOKEN));
    }
}
