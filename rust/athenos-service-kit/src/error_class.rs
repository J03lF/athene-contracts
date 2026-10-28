//! The classes every failure maps onto (step 3.4), shared by the services
//! and the kit's clients.
//!
//! A class says what the caller can do about a failure, never what broke;
//! each service answers each class with exactly one status and code, and
//! the cause stays in the log. athenos's `ErrorKind` is this type.

/// What a failure means for whoever asked, whatever produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// The input breaks a rule. Changing the input helps.
    Invalid,
    /// Nobody is signed in, or the credentials were refused.
    Unauthenticated,
    /// The caller may not do this.
    Forbidden,
    /// It is not there (or the caller may not know that it is).
    NotFound,
    /// It clashes with what is there: a taken key, a row still referenced.
    Conflict,
    /// Too many requests from this caller. Waiting helps.
    RateLimited,
    /// A dependency could not answer in time. Trying again later can help.
    Unavailable,
    /// A bug or a broken invariant. Trying again will not help.
    Internal,
}

impl ErrorClass {
    /// Whether the same request can succeed later unchanged.
    pub fn is_retryable(self) -> bool {
        matches!(self, Self::Unavailable | Self::RateLimited)
    }

    /// The class of a dependency's answer with HTTP `status` (a failure:
    /// 2xx/3xx are not passed here).
    pub fn of_status(status: u16) -> Self {
        match status {
            400 | 422 => Self::Invalid,
            401 => Self::Unauthenticated,
            403 => Self::Forbidden,
            404 | 410 => Self::NotFound,
            409 | 412 => Self::Conflict,
            429 => Self::RateLimited,
            // A 5xx, and a 408 / 504 timeout: the dependency is in trouble,
            // not this request.
            408 | 500..=599 => Self::Unavailable,
            // Any other 4xx is a request this service got wrong.
            _ => Self::Internal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ErrorClass;

    #[test]
    fn statuses_map_onto_classes() {
        for (status, class) in [
            (400, ErrorClass::Invalid),
            (422, ErrorClass::Invalid),
            (401, ErrorClass::Unauthenticated),
            (403, ErrorClass::Forbidden),
            (404, ErrorClass::NotFound),
            (410, ErrorClass::NotFound),
            (409, ErrorClass::Conflict),
            (412, ErrorClass::Conflict),
            (429, ErrorClass::RateLimited),
            (408, ErrorClass::Unavailable),
            (500, ErrorClass::Unavailable),
            (503, ErrorClass::Unavailable),
            (504, ErrorClass::Unavailable),
            (418, ErrorClass::Internal),
        ] {
            assert_eq!(ErrorClass::of_status(status), class, "{status}");
        }
        assert!(ErrorClass::Unavailable.is_retryable());
        assert!(ErrorClass::RateLimited.is_retryable());
        assert!(!ErrorClass::Internal.is_retryable());
    }
}
