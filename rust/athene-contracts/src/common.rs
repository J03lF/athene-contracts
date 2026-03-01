use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Generic API response envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ApiResponse<T> {
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

/// Generic list response metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ListMeta {
    pub count: usize,
}

impl ListMeta {
    pub fn new(count: usize) -> Self {
        Self { count }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ListResponse<T> {
    pub data: Vec<T>,
    pub meta: ListMeta,
}

impl<T> ListResponse<T> {
    pub fn new(data: Vec<T>) -> Self {
        let count = data.len();
        Self {
            data,
            meta: ListMeta::new(count),
        }
    }
}

/// Canonical API error payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            request_id: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ApiErrorEnvelope {
    pub error: ApiError,
}

impl ApiErrorEnvelope {
    pub fn new(error: ApiError) -> Self {
        Self { error }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub enum ApiErrorCode {
    #[serde(rename = "ATH-API-GW-001")]
    GatewayRequestFailed,
    #[serde(rename = "ATH-API-VAL-001")]
    ValidationFailed,
    #[serde(rename = "ATH-API-REQ-001")]
    InvalidRequest,
    #[serde(rename = "ATH-API-RATE-001")]
    RateLimitExceeded,
    #[serde(rename = "ATH-API-AUTH-001")]
    Unauthorized,
    #[serde(rename = "ATH-API-AUTH-002")]
    Forbidden,
}

impl ApiErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GatewayRequestFailed => "ATH-API-GW-001",
            Self::ValidationFailed => "ATH-API-VAL-001",
            Self::InvalidRequest => "ATH-API-REQ-001",
            Self::RateLimitExceeded => "ATH-API-RATE-001",
            Self::Unauthorized => "ATH-API-AUTH-001",
            Self::Forbidden => "ATH-API-AUTH-002",
        }
    }
}

impl std::fmt::Display for ApiErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
