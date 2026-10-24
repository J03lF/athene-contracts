//! Tracing for every service, and keeping secrets out of it (BE-13, S4).
//!
//! [`init`] installs one subscriber: an env filter (`RUST_LOG`, default
//! `info`), text lines or JSON lines (`<PREFIX>_LOG_FORMAT=json`), and —
//! with the `otel` feature, when `ATHENE_OTEL_ENABLED` or
//! `OTEL_EXPORTER_OTLP_ENDPOINT` says so — OTLP export with the W3C trace
//! context propagator. The returned [`TracingGuard`] flushes the exporter
//! when it is dropped at the end of `main` (spans used to be lost on
//! shutdown).
//!
//! [`redact`] is how a log line refers to a secret or a person: never the
//! value, at most a fingerprint or a truncated address.

use tracing_subscriber::layer::{Layered, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Layer, Registry};

/// What [`init`] reads.
#[derive(Debug, Clone, Copy)]
pub struct TracingOptions<'a> {
    pub service_name: &'static str,
    /// Variables that select the format (`json` or `text`), first set wins.
    pub format_vars: &'a [&'a str],
    /// Variables that turn OTLP export on (`1`/`true`);
    /// `OTEL_EXPORTER_OTLP_ENDPOINT` always does.
    pub otel_vars: &'a [&'a str],
}

impl<'a> TracingOptions<'a> {
    pub fn new(service_name: &'static str) -> Self {
        Self {
            service_name,
            format_vars: &["LOG_FORMAT"],
            otel_vars: &["ATHENE_OTEL_ENABLED"],
        }
    }

    #[must_use]
    pub fn format_vars(mut self, vars: &'a [&'a str]) -> Self {
        self.format_vars = vars;
        self
    }

    #[must_use]
    pub fn otel_vars(mut self, vars: &'a [&'a str]) -> Self {
        self.otel_vars = vars;
        self
    }
}

/// The log line format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Text,
    /// One JSON object per line, the event's fields at the top level, the
    /// current span (with `request_id`) as `span`.
    Json,
}

impl LogFormat {
    pub fn from_lookup(lookup: &dyn Fn(&str) -> Option<String>, vars: &[&str]) -> Self {
        let json = vars
            .iter()
            .find_map(|var| lookup(var))
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("json"));
        if json {
            Self::Json
        } else {
            Self::Text
        }
    }
}

/// Could not set up the exporter.
#[derive(Debug, thiserror::Error)]
#[error("tracing setup failed: {0}")]
pub struct InitError(String);

/// Keeps the exporter alive; flushes it when dropped.
#[derive(Default)]
pub struct TracingGuard {
    #[cfg(feature = "otel")]
    provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
}

impl std::fmt::Debug for TracingGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TracingGuard").finish_non_exhaustive()
    }
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        #[cfg(feature = "otel")]
        if let Some(provider) = self.provider.take() {
            if let Err(err) = provider.shutdown() {
                tracing::warn!(error = %err, "flushing the trace exporter failed");
            }
        }
    }
}

type Base = Layered<EnvFilter, Registry>;
type BoxedLayer = Box<dyn Layer<Base> + Send + Sync>;

/// Installs the subscriber (a second call, e.g. in tests, is a no-op).
pub fn init(options: TracingOptions<'_>) -> Result<TracingGuard, InitError> {
    init_from(&|name| std::env::var(name).ok(), options)
}

/// [`init`] through `lookup` instead of the process environment.
pub fn init_from(
    lookup: &dyn Fn(&str) -> Option<String>,
    options: TracingOptions<'_>,
) -> Result<TracingGuard, InitError> {
    let filter = lookup("RUST_LOG")
        .and_then(|raw| EnvFilter::try_new(raw).ok())
        .unwrap_or_else(|| EnvFilter::new("info"));
    #[cfg_attr(not(feature = "otel"), allow(unused_mut))]
    let mut layers: Vec<BoxedLayer> = vec![fmt_layer(LogFormat::from_lookup(
        lookup,
        options.format_vars,
    ))];
    #[cfg_attr(not(feature = "otel"), allow(unused_mut))]
    let mut guard = TracingGuard::default();

    let otel_asked = lookup("OTEL_EXPORTER_OTLP_ENDPOINT").is_some()
        || options.otel_vars.iter().any(|var| {
            lookup(var).is_some_and(|value| {
                let value = value.trim();
                value == "1" || value.eq_ignore_ascii_case("true")
            })
        });
    if otel_asked {
        #[cfg(feature = "otel")]
        {
            let (layer, provider) = otel_layer(lookup, options.service_name)?;
            layers.push(layer);
            guard.provider = Some(provider);
        }
    }

    let _ = Registry::default().with(filter).with(layers).try_init();
    #[cfg(not(feature = "otel"))]
    if otel_asked {
        tracing::debug!(
            service = options.service_name,
            "OTLP export was asked for, but this build has no exporter"
        );
    }
    Ok(guard)
}

fn fmt_layer(format: LogFormat) -> BoxedLayer {
    match format {
        LogFormat::Text => fmt::layer().with_target(false).boxed(),
        LogFormat::Json => fmt::layer()
            .json()
            .flatten_event(true)
            .with_current_span(true)
            .with_span_list(false)
            .with_target(false)
            .boxed(),
    }
}

#[cfg(feature = "otel")]
fn otel_layer(
    lookup: &dyn Fn(&str) -> Option<String>,
    service_name: &'static str,
) -> Result<(BoxedLayer, opentelemetry_sdk::trace::SdkTracerProvider), InitError> {
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_otlp::WithExportConfig;

    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );
    let builder = opentelemetry_otlp::SpanExporter::builder().with_tonic();
    let exporter = match lookup("OTEL_EXPORTER_OTLP_ENDPOINT") {
        Some(url) => builder.with_endpoint(url).build(),
        None => builder.build(),
    }
    .map_err(|err| InitError(format!("otlp exporter: {err}")))?;
    let resource = opentelemetry_sdk::Resource::builder_empty()
        .with_service_name(service_name)
        .build();
    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();
    let tracer = provider.tracer(service_name);
    opentelemetry::global::set_tracer_provider(provider.clone());
    Ok((
        tracing_opentelemetry::layer().with_tracer(tracer).boxed(),
        provider,
    ))
}

/// What a service answers on `/metrics/internal`, the host's scrape
/// (every 15 s): who it is, since when, and whether it is draining. A
/// service without its own metrics serves this instead of a 404 the
/// error log would record at every scrape.
#[derive(Debug, Clone)]
pub struct RuntimeMetrics {
    service: &'static str,
    version: &'static str,
    started: std::time::Instant,
    started_at: u64,
}

/// One scrape.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RuntimeSnapshot {
    pub service: &'static str,
    pub version: &'static str,
    /// Unix seconds.
    pub started_at: u64,
    pub uptime_seconds: u64,
    pub draining: bool,
}

impl RuntimeMetrics {
    pub fn new(service: &'static str, version: &'static str) -> Self {
        let started_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_secs())
            .unwrap_or_default();
        Self {
            service,
            version,
            started: std::time::Instant::now(),
            started_at,
        }
    }

    pub fn snapshot(&self, drain: &crate::shutdown::Drain) -> RuntimeSnapshot {
        RuntimeSnapshot {
            service: self.service,
            version: self.version,
            started_at: self.started_at,
            uptime_seconds: self.started.elapsed().as_secs(),
            draining: drain.is_draining(),
        }
    }
}

/// How logs refer to what must not be in them.
pub mod redact {
    use std::fmt;
    use std::net::IpAddr;

    use sha2::{Digest, Sha256};

    pub const REDACTED: &str = "<redacted>";

    /// Prints as `<redacted>`, whatever it holds.
    #[derive(Clone, Copy)]
    pub struct Redacted<T>(pub T);

    impl<T> fmt::Display for Redacted<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(REDACTED)
        }
    }

    impl<T> fmt::Debug for Redacted<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(REDACTED)
        }
    }

    /// A short, stable stand-in for a secret, to tell two values apart in
    /// a log without revealing either: `sha256:` and 12 hex digits.
    pub fn fingerprint(secret: &str) -> String {
        let digest = Sha256::digest(secret.as_bytes());
        format!("sha256:{}", &crate::rate_limit::hex(&digest)[..12])
    }

    /// An address cut to its network: IPv4 to /24, IPv6 to /48.
    pub fn ip(ip: IpAddr) -> String {
        match ip.to_canonical() {
            IpAddr::V4(v4) => {
                let [a, b, c, _] = v4.octets();
                format!("{a}.{b}.{c}.0/24")
            }
            IpAddr::V6(v6) => {
                let s = v6.segments();
                format!("{:x}:{:x}:{:x}::/48", s[0], s[1], s[2])
            }
        }
    }

    /// An e-mail address with the local part reduced to its first letter.
    pub fn email(address: &str) -> String {
        match address.split_once('@') {
            Some((local, domain)) => {
                let first: String = local.chars().take(1).collect();
                format!("{first}***@{domain}")
            }
            None => REDACTED.to_string(),
        }
    }

    /// Free text as it may be logged (an error message, a body excerpt):
    /// every `/invite/<token>` in it becomes `/invite/<redacted>`, the token
    /// being whatever follows up to the next `/`, `?`, `#`, quote, bracket,
    /// comma, semicolon or whitespace. For texts that quote a URL — the
    /// host's client error names its target (3.8a, N1).
    pub fn text(text: &str) -> std::borrow::Cow<'_, str> {
        const MARK: &str = "/invite/";
        let ends = |c: char| {
            matches!(
                c,
                '/' | '?' | '#' | '"' | '\'' | ')' | ']' | '>' | ',' | ';'
            ) || c.is_whitespace()
        };
        let mut out = String::new();
        let mut rest = text;
        let mut changed = false;
        while let Some(at) = rest.find(MARK) {
            let (before, after) = rest.split_at(at + MARK.len());
            let len = after.find(ends).unwrap_or(after.len());
            out.push_str(before);
            if len > 0 {
                out.push_str(REDACTED);
                changed = true;
            }
            rest = &after[len..];
        }
        if !changed {
            return std::borrow::Cow::Borrowed(text);
        }
        out.push_str(rest);
        std::borrow::Cow::Owned(out)
    }

    /// A request path as it may be logged: the secret segment of an invite
    /// link (`/invite/<token>`, `/api/v1/invite/<token>/join`) becomes
    /// `<redacted>` (step 3.8a). The token is the whole credential of a
    /// link invite; the access logs, the `http` span and the refusal logs
    /// all print paths, so every one of them goes through here.
    pub fn path(path: &str) -> std::borrow::Cow<'_, str> {
        let segments: Vec<&str> = path.split('/').collect();
        let secret =
            |idx: usize| idx > 0 && segments[idx - 1] == "invite" && !segments[idx].is_empty();
        if !(0..segments.len()).any(secret) {
            return std::borrow::Cow::Borrowed(path);
        }
        std::borrow::Cow::Owned(
            segments
                .iter()
                .enumerate()
                .map(|(idx, segment)| if secret(idx) { REDACTED } else { segment })
                .collect::<Vec<_>>()
                .join("/"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::redact;
    use super::*;

    #[test]
    fn the_format_is_chosen_by_the_first_variable_set() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| value.to_string())
            }
        };
        assert_eq!(LogFormat::from_lookup(&env(&[]), &["A"]), LogFormat::Text);
        assert_eq!(
            LogFormat::from_lookup(&env(&[("A", "JSON")]), &["A"]),
            LogFormat::Json
        );
        assert_eq!(
            LogFormat::from_lookup(&env(&[("A", "text"), ("B", "json")]), &["A", "B"]),
            LogFormat::Text
        );
    }

    #[test]
    fn init_twice_is_harmless() {
        let lookup = |_: &str| None;
        assert!(init_from(&lookup, TracingOptions::new("kit-test")).is_ok());
        assert!(init_from(&lookup, TracingOptions::new("kit-test")).is_ok());
    }

    #[test]
    fn the_runtime_snapshot_reports_draining() {
        let metrics = RuntimeMetrics::new("kit-test", "1.2.3");
        let drain = crate::shutdown::Drain::default();
        let snapshot = metrics.snapshot(&drain);
        assert_eq!((snapshot.service, snapshot.version), ("kit-test", "1.2.3"));
        assert!(!snapshot.draining && snapshot.started_at > 0);
        drain.begin_shutdown();
        assert!(metrics.snapshot(&drain).draining);
    }

    #[test]
    fn redaction_never_shows_the_value() {
        let secret = "setup_0123456789abcdef0123456789abcdef";
        assert_eq!(format!("{}", redact::Redacted(secret)), "<redacted>");
        assert_eq!(format!("{:?}", redact::Redacted(secret)), "<redacted>");
        let fp = redact::fingerprint(secret);
        assert!(fp.starts_with("sha256:") && fp.len() == 19, "{fp}");
        assert!(!fp.contains("0123456789abcdef"));
        assert_eq!(fp, redact::fingerprint(secret), "stable");
        assert_eq!(
            redact::ip("203.0.113.77".parse().unwrap()),
            "203.0.113.0/24"
        );
        assert_eq!(
            redact::ip("::ffff:10.1.2.3".parse().unwrap()),
            "10.1.2.0/24"
        );
        assert_eq!(
            redact::ip("2001:db8:1:2::5".parse().unwrap()),
            "2001:db8:1::/48"
        );
        assert_eq!(redact::email("joel@example.com"), "j***@example.com");
        assert_eq!(redact::email("not-an-address"), "<redacted>");
    }

    #[test]
    fn an_invite_token_in_free_text_is_masked() {
        let token = "AbCdEfGh0123456789AbCdEfGh012345";
        let text = format!(
            "error sending request for url (http://127.0.0.1:41001/invite/{token}/join?x=1): \
             connection refused; retried /api/v1/invite/{token} and \"/invite/{token}\""
        );
        let masked = redact::text(&text);
        assert!(!masked.contains(token), "{masked}");
        assert_eq!(
            masked,
            "error sending request for url (http://127.0.0.1:41001/invite/<redacted>/join?x=1): \
             connection refused; retried /api/v1/invite/<redacted> and \"/invite/<redacted>\""
        );
        for plain in [
            "no path here",
            "/workspaces/7e8f/invites/1c2d/rotate",
            "/invite/",
            "",
        ] {
            assert!(
                matches!(redact::text(plain), std::borrow::Cow::Borrowed(p) if p == plain),
                "{plain:?} must stay as it is"
            );
        }
    }

    #[test]
    fn an_invite_token_in_a_path_is_masked() {
        let token = "AbCdEfGh0123456789AbCdEfGh012345";
        for (path, logged) in [
            (format!("/invite/{token}"), "/invite/<redacted>"),
            (format!("/invite/{token}/join"), "/invite/<redacted>/join"),
            (
                format!("/api/v1/invite/{token}/join"),
                "/api/v1/invite/<redacted>/join",
            ),
        ] {
            assert_eq!(redact::path(&path), logged);
        }
        for path in [
            "/workspaces/7e8f/invites/1c2d/rotate",
            "/invite/",
            "/work/tickets/7e8f",
            "/",
        ] {
            assert!(
                matches!(redact::path(path), std::borrow::Cow::Borrowed(p) if p == path),
                "{path} must stay as it is"
            );
        }
    }
}
