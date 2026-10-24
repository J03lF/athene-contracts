//! Small pieces the layers share.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use http::header::{HeaderValue, CONTENT_TYPE};
use http::{Response, StatusCode};

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

/// At most `max` characters of `text`, for a log line.
pub(crate) fn excerpt(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}
