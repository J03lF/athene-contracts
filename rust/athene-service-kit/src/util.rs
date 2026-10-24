//! Small pieces the layers share.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use http::header::{HeaderName, HeaderValue, CONTENT_TYPE, RETRY_AFTER};
use http::{Response, StatusCode};
use serde::Serialize;

/// A boxed, sendable future.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// How a layer answers a request it refuses: a function of what was refused
/// (`R`) to the inner service's response type. Each module passes its own,
/// so the refusal carries that module's error envelope.
pub type Render<R, B> = Arc<dyn Fn(&R) -> Response<B> + Send + Sync>;

/// A JSON response with `status`, for the kit's default refusals.
pub(crate) fn json_response<B: From<String>>(status: StatusCode, body: String) -> Response<B> {
    let mut response = Response::new(B::from(body));
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    response
}

#[derive(Serialize)]
struct Envelope<'a> {
    error: Body<'a>,
}

#[derive(Serialize)]
struct Body<'a> {
    code: &'static str,
    message: &'static str,
    message_key: &'static str,
    details: Details,
    retryable: bool,
    request_id: &'a str,
}

#[derive(Serialize)]
struct Details {
    reason: &'static str,
    retry_after_seconds: u64,
}

/// A retryable `503` in athene's envelope (`code`, `message`,
/// `message_key: errors.unavailable`, `details {reason,
/// retry_after_seconds}`, `retryable: true`, `request_id`) with
/// `Retry-After` and `x-request-id`: the kit's default answer when a
/// service cannot serve a request for a reason that is not the end user's
/// (draining, a refused service credential).
pub(crate) fn unavailable_response<B: From<String>>(
    code: &'static str,
    message: &'static str,
    reason: &'static str,
    retry_after_seconds: u64,
) -> Response<B> {
    let request_id =
        crate::request_id::current().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let body = serde_json::to_string(&Envelope {
        error: Body {
            code,
            message,
            message_key: "errors.unavailable",
            details: Details {
                reason,
                retry_after_seconds,
            },
            retryable: true,
            request_id: &request_id,
        },
    })
    .unwrap_or_default();
    let mut response = json_response(StatusCode::SERVICE_UNAVAILABLE, body);
    response
        .headers_mut()
        .insert(RETRY_AFTER, HeaderValue::from(retry_after_seconds));
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-request-id"), value);
    }
    response
}

/// At most `max` characters of `text`, for a log line.
pub(crate) fn excerpt(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}
