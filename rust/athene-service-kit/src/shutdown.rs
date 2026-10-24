//! Graceful shutdown: on `SIGTERM` or Ctrl-C a service starts draining —
//! readiness goes to 503 and every request but the probes is answered
//! `503 ATH-SYS-005` with `Retry-After`, retryable and nothing about the
//! caller (athene's answer, now everyone's default) — while in-flight
//! requests finish.
//!
//! ```ignore
//! let drain = Arc::new(Drain::default());
//! axum::serve(listener, app)
//!     .with_graceful_shutdown(shutdown::drain_on_signal(Arc::clone(&drain), "athene"))
//!     .await?;
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use http::{Request, Response};
use tower::{Layer, Service};

use crate::service_auth::PROBE_PATHS;
use crate::util::{unavailable_response, BoxFuture, Render};

/// Whether the service is draining. Shared by the drain layer, `/ready`
/// and the shutdown future.
#[derive(Debug, Default)]
pub struct Drain {
    draining: AtomicBool,
}

impl Drain {
    pub fn begin_shutdown(&self) {
        self.draining.store(true, Ordering::SeqCst);
    }

    pub fn is_ready(&self) -> bool {
        !self.is_draining()
    }

    pub fn is_draining(&self) -> bool {
        self.draining.load(Ordering::SeqCst)
    }
}

/// Resolves on Ctrl-C or `SIGTERM`.
pub async fn signal() {
    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(err) = result {
                tracing::warn!(error = %err, "failed to listen for ctrl_c shutdown signal");
                std::future::pending::<()>().await;
            }
        }
        () = terminate() => {}
    }
}

#[cfg(unix)]
async fn terminate() {
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        Ok(mut signal) => {
            signal.recv().await;
        }
        Err(err) => {
            tracing::warn!(error = %err, "failed to listen for SIGTERM shutdown signal");
            std::future::pending::<()>().await;
        }
    }
}

#[cfg(not(unix))]
async fn terminate() {
    std::future::pending::<()>().await;
}

/// Waits for [`signal`], then drains: the future for
/// `with_graceful_shutdown`.
pub async fn drain_on_signal(drain: Arc<Drain>, service: &str) {
    signal().await;
    drain.begin_shutdown();
    tracing::info!("{service} entering drain mode");
}

/// What the drain layer refused.
#[derive(Debug, Clone, Copy)]
pub struct Draining;

/// `503 ATH-SYS-005` with `Retry-After: 1`, the request id and the retry
/// details — the same envelope athene's `AppError` writes.
pub fn draining_response<B: From<String>>(_: &Draining) -> Response<B> {
    unavailable_response("ATH-SYS-005", "Service is restarting", "draining", 1)
}

/// Refuses every request but the exempt paths while draining.
pub struct DrainLayer<B> {
    drain: Arc<Drain>,
    exempt: Arc<Vec<String>>,
    reject: Render<Draining, B>,
}

impl<B> Clone for DrainLayer<B> {
    fn clone(&self) -> Self {
        Self {
            drain: Arc::clone(&self.drain),
            exempt: Arc::clone(&self.exempt),
            reject: Arc::clone(&self.reject),
        }
    }
}

impl<B> DrainLayer<B> {
    /// Exempt: [`PROBE_PATHS`].
    pub fn new(drain: Arc<Drain>, reject: Render<Draining, B>) -> Self {
        Self {
            drain,
            exempt: Arc::new(PROBE_PATHS.iter().map(|path| (*path).to_string()).collect()),
            reject,
        }
    }

    #[must_use]
    pub fn with_exempt_paths(mut self, paths: &[&str]) -> Self {
        self.exempt = Arc::new(paths.iter().map(|path| (*path).to_string()).collect());
        self
    }
}

impl<B: From<String> + 'static> DrainLayer<B> {
    /// With [`draining_response`].
    pub fn with_default_answer(drain: Arc<Drain>) -> Self {
        Self::new(drain, Arc::new(draining_response::<B>))
    }
}

impl<S, B> Layer<S> for DrainLayer<B> {
    type Service = DrainService<S, B>;

    fn layer(&self, inner: S) -> Self::Service {
        DrainService {
            inner,
            layer: self.clone(),
        }
    }
}

pub struct DrainService<S, B> {
    inner: S,
    layer: DrainLayer<B>,
}

impl<S: Clone, B> Clone for DrainService<S, B> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            layer: self.layer.clone(),
        }
    }
}

impl<S, ReqB, ResB> Service<Request<ReqB>> for DrainService<S, ResB>
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
        let exempt = self
            .layer
            .exempt
            .iter()
            .any(|path| path == request.uri().path());
        if self.layer.drain.is_draining() && !exempt {
            return Box::pin(std::future::ready(Ok((self.layer.reject)(&Draining))));
        }
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(async move { inner.call(request).await })
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::*;
    use http::header::RETRY_AFTER;
    use http::StatusCode;
    use tower::ServiceExt;

    async fn ok(_: Request<()>) -> Result<Response<String>, Infallible> {
        Ok(Response::new("ok".to_string()))
    }

    #[tokio::test]
    async fn while_draining_everything_but_the_probes_is_refused() {
        let drain = Arc::new(Drain::default());
        let service = DrainLayer::<String>::with_default_answer(Arc::clone(&drain))
            .layer(tower::service_fn(ok));
        let call = |path: &'static str| {
            let service = service.clone();
            async move {
                service
                    .oneshot(Request::builder().uri(path).body(()).unwrap())
                    .await
                    .unwrap()
            }
        };
        assert_eq!(call("/work/x").await.status(), StatusCode::OK);
        assert!(drain.is_ready());

        drain.begin_shutdown();
        assert!(!drain.is_ready());
        let refused = call("/work/x").await;
        assert_eq!(refused.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(refused.headers()[RETRY_AFTER], "1");
        let body: serde_json::Value = serde_json::from_str(refused.body()).unwrap();
        assert_eq!(body["error"]["code"], "ATH-SYS-005");
        assert_eq!(body["error"]["message_key"], "errors.unavailable");
        assert_eq!(body["error"]["retryable"], true);
        assert_eq!(body["error"]["details"]["reason"], "draining");
        assert_eq!(
            refused.headers()["x-request-id"],
            body["error"]["request_id"].as_str().unwrap()
        );
        for probe in PROBE_PATHS {
            assert_eq!(call(probe).await.status(), StatusCode::OK, "{probe}");
        }
    }
}
