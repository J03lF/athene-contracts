//! The Athenos service kit (roadmap step 3.6, Q-056 / BE-14).
//!
//! Every Athenos Rust service needs the same behaviour at its edge, and until
//! now each one carried its own copy of it — with the drift that implies:
//! notification-service lacked the auth middleware (BE-01), athenos-api keyed
//! its limiter on a header any client can set and never evicted (SEC-19),
//! service auth switched itself off without a token (BE-20/SEC-08), and the
//! gateway and the core each resolved the same user (Q-009). This crate is
//! the one copy:
//!
//! | module | what |
//! |---|---|
//! | [`config`] | `bind_addr()` (`FENRIR_SERVICE_ADDR`, loopback default), `*_FILE` secrets, [`config::Secret`], [`config::password_from`] |
//! | [`mail`] | whether an address can receive mail at all (no `.local`, `.test`, `example.com`, …) |
//! | [`client_ip`] | client address from trusted proxies only: layer and extractor |
//! | [`rate_limit`] | bounded per-key limiter with eviction, keys by IP / verified token / account, a layer per [`rate_limit::RateClass`], a cap on open streams per caller |
//! | [`request_id`] | accept or mint `x-request-id`, `http` span, response header, [`request_id::current`] |
//! | [`service_auth`] | `x-fenrir-service-token`, report or enforce, fail-closed, constant time, open probes |
//! | [`principal`] | the gateway signs `x-athenos-principal`, the core verifies it (identity once at the edge) |
//! | [`retry`] | when a failed call may be sent again: never sent, or a read that timed out (3.6c) |
//! | [`upstream`] | the client to the host gateway: budget, breaker per target with half-open, idempotent retries only |
//! | [`outbound`] | the client for user-supplied URLs (webhooks): SSRF guard after DNS, scheme/port allowlist, size cap |
//! | [`observability`] | tracing init (text or JSON, env filter, optional OTLP), [`observability::redact`] |
//! | [`shutdown`] | signal, [`shutdown::Drain`] and the layer that answers `ATH-SYS-005` while draining |
//! | [`body_fault`] | what a request body that does not fit its contract got wrong |
//! | [`error_class`] | the error classes every failure maps onto |
//!
//! The layers are tower layers over `http` types: they take the inner
//! service's response body type as a parameter and a module-supplied
//! function for their refusal, so each service keeps its own error envelope
//! byte for byte while the decision lives here.

pub mod body_fault;
pub mod client_ip;
pub mod config;
pub mod error_class;
pub mod mail;
pub mod observability;
pub mod outbound;
pub mod principal;
pub mod rate_limit;
pub mod request_id;
pub mod retry;
pub mod service_auth;
pub mod shutdown;
pub mod upstream;

mod util;

pub use client_ip::{ClientIp, ClientIpLayer, TrustedProxies};
pub use config::{ConfigError, Secret};
pub use error_class::ErrorClass;
pub use rate_limit::{
    ConcurrencyLimitLayer, ConcurrencyLimiter, RateClass, RateLimitLayer, RateLimiter, RateRule,
    SlotHolder, SlotRefusal, VerifiedCallers,
};
pub use request_id::RequestIdLayer;
pub use service_auth::{
    AuthenticatedService, ServiceAuth, ServiceAuthConfig, ServiceAuthLayer, ServiceAuthMode,
    ServiceCredential,
};
pub use shutdown::{Drain, DrainLayer};
pub use util::{BoxFuture, Render};
