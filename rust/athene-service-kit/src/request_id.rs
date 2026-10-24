//! One request id per request, from the gateway to auth-service (BE-13).
//!
//! The layer accepts the caller's `x-request-id` when it looks like an id
//! (at most 128 of `A-Za-z0-9-_.:/+`), else mints a UUID, and
//! - writes it back into the request (so every outbound call that forwards
//!   headers carries it) and stores [`RequestId`] as an extension;
//! - runs the rest of the request in an `http` span with `request_id`,
//!   `method` and `path`, so every log line of the request has it — the
//!   layer therefore sits outside the trace and access-log layers;
//! - makes it [`current`] for the task, for error envelopes;
//! - sets it on the response unless the response already has one.
//!
//! The same for `x-correlation-id` (defaulting to the request id), unless
//! the layer is built [`RequestIdLayer::without_correlation`].

use std::task::{Context, Poll};

use http::header::{HeaderName, HeaderValue};
use http::{Request, Response};
use tower::{Layer, Service};
use tracing::Instrument;
use uuid::Uuid;

use crate::util::BoxFuture;

pub const REQUEST_ID_HEADER: &str = "x-request-id";
pub const CORRELATION_ID_HEADER: &str = "x-correlation-id";

tokio::task_local! {
    static CURRENT_REQUEST_ID: String;
}

/// The id of the request this task serves, if it serves one.
pub fn current() -> Option<String> {
    CURRENT_REQUEST_ID.try_with(Clone::clone).ok()
}

/// The request's id, as a request extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestId(pub String);

/// Whether `value` may be used as an id as sent.
pub fn is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':' | '/' | '+'))
}

/// A header value usable as an id.
pub fn accepted(value: Option<&HeaderValue>) -> Option<String> {
    value
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| is_valid(value))
        .map(str::to_string)
}

#[derive(Debug, Clone, Copy)]
pub struct RequestIdLayer {
    correlation: bool,
}

impl RequestIdLayer {
    pub fn new() -> Self {
        Self { correlation: true }
    }

    /// Only `x-request-id`; `x-correlation-id` is left as it is.
    pub fn without_correlation() -> Self {
        Self { correlation: false }
    }
}

impl Default for RequestIdLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for RequestIdLayer {
    type Service = RequestIdService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestIdService {
            inner,
            correlation: self.correlation,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestIdService<S> {
    inner: S,
    correlation: bool,
}

impl<S, ReqB, ResB> Service<Request<ReqB>> for RequestIdService<S>
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
        let request_name = HeaderName::from_static(REQUEST_ID_HEADER);
        let correlation_name = HeaderName::from_static(CORRELATION_ID_HEADER);

        let request_id = accepted(request.headers().get(&request_name))
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let correlation_id = self.correlation.then(|| {
            accepted(request.headers().get(&correlation_name)).unwrap_or_else(|| request_id.clone())
        });

        if let Ok(value) = HeaderValue::from_str(&request_id) {
            request.headers_mut().insert(request_name.clone(), value);
        }
        if let Some(Ok(value)) = correlation_id.as_deref().map(HeaderValue::from_str) {
            request
                .headers_mut()
                .insert(correlation_name.clone(), value);
        }
        request
            .extensions_mut()
            .insert(RequestId(request_id.clone()));

        let span = tracing::info_span!(
            "http",
            request_id = %request_id,
            method = %request.method(),
            path = %crate::observability::redact::path(request.uri().path()),
        );
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(async move {
            let mut response = CURRENT_REQUEST_ID
                .scope(request_id.clone(), inner.call(request).instrument(span))
                .await?;
            if !response.headers().contains_key(&request_name) {
                if let Ok(value) = HeaderValue::from_str(&request_id) {
                    response.headers_mut().insert(request_name, value);
                }
            }
            if let Some(correlation_id) = correlation_id {
                if !response.headers().contains_key(&correlation_name) {
                    if let Ok(value) = HeaderValue::from_str(&correlation_id) {
                        response.headers_mut().insert(correlation_name, value);
                    }
                }
            }
            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;
    use tower::ServiceExt;

    #[test]
    fn ids_are_accepted_as_the_gateway_accepts_them() {
        for (raw, ok) in [
            ("3f0c9a52-1b7e-4d1a-9f3e-2b6c8d7e9a10", true),
            ("  req-42  ", true),
            ("trace/span+1", true),
            ("", false),
            ("has space", false),
            ("<script>", false),
            ("id\u{e9}", false),
        ] {
            let value = HeaderValue::from_str(raw).ok();
            assert_eq!(accepted(value.as_ref()).is_some(), ok, "{raw:?}");
        }
        let long = "a".repeat(129);
        assert!(accepted(HeaderValue::from_str(&long).ok().as_ref()).is_none());
    }

    /// Echoes what the handler saw: the request's id header and `current()`.
    async fn echo(request: Request<()>) -> Result<Response<String>, Infallible> {
        let header = request
            .headers()
            .get(REQUEST_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        assert_eq!(current().as_deref(), Some(header.as_str()));
        assert_eq!(
            request
                .extensions()
                .get::<RequestId>()
                .map(|id| id.0.clone()),
            Some(header.clone())
        );
        Ok(Response::new(header))
    }

    #[tokio::test]
    async fn a_good_id_is_kept_and_a_bad_one_replaced() {
        let service = RequestIdLayer::new().layer(tower::service_fn(echo));
        let mut request = Request::new(());
        request
            .headers_mut()
            .insert(REQUEST_ID_HEADER, HeaderValue::from_static("req-42"));
        let response = service.clone().oneshot(request).await.unwrap();
        assert_eq!(response.body(), "req-42");
        assert_eq!(response.headers()[REQUEST_ID_HEADER], "req-42");
        assert_eq!(response.headers()[CORRELATION_ID_HEADER], "req-42");

        let mut request = Request::new(());
        request
            .headers_mut()
            .insert(REQUEST_ID_HEADER, HeaderValue::from_static("<bad id>"));
        let response = service.oneshot(request).await.unwrap();
        assert_ne!(response.body(), "<bad id>");
        assert!(Uuid::parse_str(response.body()).is_ok());
        assert_eq!(current(), None, "only inside the request");
    }

    #[tokio::test]
    async fn a_response_that_set_its_own_id_keeps_it() {
        let service = RequestIdLayer::without_correlation().layer(tower::service_fn(
            |_request: Request<()>| async move {
                let mut response = Response::new(String::new());
                response
                    .headers_mut()
                    .insert(REQUEST_ID_HEADER, HeaderValue::from_static("own"));
                Ok::<_, Infallible>(response)
            },
        ));
        let response = service.oneshot(Request::new(())).await.unwrap();
        assert_eq!(response.headers()[REQUEST_ID_HEADER], "own");
        assert!(response.headers().get(CORRELATION_ID_HEADER).is_none());
    }
}
