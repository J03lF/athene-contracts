//! When a failed call may be sent again (roadmap 3.6c, the Q-022 class).
//!
//! The rule is the host's (`fenrir/src/services/module/gateway.rs`,
//! `repeat_is_safe`, 5.6):
//!
//! - a call that **never left** (no connection could be made, or the host
//!   said it did not dispatch it) may be repeated, whatever it does;
//! - a call that **timed out** may have run: it is repeated only when it
//!   only reads (`GET`, `HEAD`, `OPTIONS`, or a read statement);
//! - **anything else** is an answer, or a break after the request went out:
//!   never repeated. A 5xx is an answer, and a POST that failed half-way may
//!   have written.
//!
//! Each service maps its own transport's errors onto [`CallFailure`]; the
//! decision lives here once. [`crate::upstream`] keeps its stricter rule for
//! the gateway (no POST/PATCH ever).

/// Why a call came back without a usable answer, as far as repeating it is
/// concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallFailure {
    /// The request never reached the other side.
    NotSent,
    /// No answer in time; the request may have run.
    TimedOut,
    /// Anything else: an error answer, or a break after sending.
    Other,
}

/// Whether a call that failed with `failure` may be sent again.
/// `read_only`: the call only reads (see [`is_read_method`]).
pub fn repeat_is_safe(read_only: bool, failure: CallFailure) -> bool {
    match failure {
        CallFailure::NotSent => true,
        CallFailure::TimedOut => read_only,
        CallFailure::Other => false,
    }
}

/// `GET`, `HEAD` and `OPTIONS`: the methods a timeout may repeat.
pub fn is_read_method(method: &str) -> bool {
    matches!(
        method.trim().to_ascii_uppercase().as_str(),
        "GET" | "HEAD" | "OPTIONS"
    )
}

/// The failure class of a `reqwest` error. A connect error (refused, no
/// route, DNS, a connect timeout) never sent the request; any other timeout
/// may have.
pub fn of_reqwest(err: &reqwest::Error) -> CallFailure {
    if err.is_connect() {
        CallFailure::NotSent
    } else if err.is_timeout() {
        CallFailure::TimedOut
    } else {
        CallFailure::Other
    }
}

/// The failure class of an I/O error on a connection being opened: the
/// kinds `connect(2)` reports when nothing was sent. Anything else (a reset,
/// a broken pipe, an EOF) may have come after the request went out.
pub fn of_io_kind(kind: std::io::ErrorKind) -> CallFailure {
    use std::io::ErrorKind;
    match kind {
        ErrorKind::NotFound
        | ErrorKind::ConnectionRefused
        | ErrorKind::AddrNotAvailable
        | ErrorKind::PermissionDenied => CallFailure::NotSent,
        ErrorKind::TimedOut => CallFailure::TimedOut,
        _ => CallFailure::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_unsent_calls_repeat_whatever_they_do() {
        assert!(repeat_is_safe(false, CallFailure::NotSent));
        assert!(repeat_is_safe(true, CallFailure::NotSent));
        assert!(!repeat_is_safe(false, CallFailure::TimedOut));
        assert!(repeat_is_safe(true, CallFailure::TimedOut));
        assert!(!repeat_is_safe(true, CallFailure::Other));
        assert!(!repeat_is_safe(false, CallFailure::Other));
    }

    #[test]
    fn read_methods_are_get_head_options() {
        for method in ["GET", "head", " options "] {
            assert!(is_read_method(method), "{method}");
        }
        for method in ["POST", "PUT", "PATCH", "DELETE", ""] {
            assert!(!is_read_method(method), "{method}");
        }
    }

    #[test]
    fn io_kinds_of_a_failed_connect_never_sent() {
        use std::io::ErrorKind;
        for kind in [
            ErrorKind::NotFound,
            ErrorKind::ConnectionRefused,
            ErrorKind::AddrNotAvailable,
            ErrorKind::PermissionDenied,
        ] {
            assert_eq!(of_io_kind(kind), CallFailure::NotSent, "{kind:?}");
        }
        assert_eq!(of_io_kind(ErrorKind::TimedOut), CallFailure::TimedOut);
        for kind in [
            ErrorKind::BrokenPipe,
            ErrorKind::ConnectionReset,
            ErrorKind::UnexpectedEof,
            ErrorKind::Other,
        ] {
            assert_eq!(of_io_kind(kind), CallFailure::Other, "{kind:?}");
        }
    }

    #[tokio::test]
    async fn a_refused_connection_is_not_sent_and_a_silent_server_times_out() {
        // A port nobody listens on: bind, learn the port, close.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(150))
            .build()
            .unwrap();
        let refused = client
            .post(format!("http://127.0.0.1:{port}/"))
            .send()
            .await
            .unwrap_err();
        assert_eq!(of_reqwest(&refused), CallFailure::NotSent);

        // Accepts, never answers.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((stream, _)) = listener.accept().await {
                held.push(stream);
            }
        });
        let silent = client
            .post(format!("http://{addr}/"))
            .send()
            .await
            .unwrap_err();
        assert_eq!(of_reqwest(&silent), CallFailure::TimedOut);
    }
}
