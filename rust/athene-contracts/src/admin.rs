use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema)]
pub struct FeatureFlagRequest {
    #[validate(length(min = 1, max = 128))]
    pub key: String,
    #[validate(length(min = 1, max = 512))]
    pub description: String,
    pub enabled: bool,
    #[serde(default)]
    pub requires_api_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema)]
pub struct ToggleRequest {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct FeatureFlagResponse {
    pub key: String,
    pub description: String,
    pub enabled: bool,
    pub requires_api_key: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema)]
pub struct CreateApiKeyRequest {
    #[validate(length(min = 1, max = 128))]
    pub name: String,
    #[validate(length(min = 1))]
    pub expires_in: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ApiKeyResponse {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub status: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ApiKeyCreatedResponse {
    pub id: String,
    pub key: String,
    pub name: String,
    pub prefix: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema)]
pub struct MaintenanceRequest {
    pub enabled: bool,
    #[validate(length(max = 500))]
    pub message: Option<String>,
    pub until: Option<String>,
    pub force_logout_at: Option<String>,
    pub dismiss_seconds: Option<i32>,
    pub warn_minutes: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema)]
pub struct UpdateSettingsRequest {
    pub registration_mode: Option<String>,
    pub max_users: Option<u32>,
    pub session_timeout_minutes: Option<u32>,
    pub allowed_email_domains: Option<Vec<String>>,
    pub maintenance_mode: Option<bool>,
    #[validate(length(max = 500))]
    pub maintenance_message: Option<String>,
    pub maintenance_until: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct SettingsResponse {
    pub registration_mode: String,
    pub max_users: u32,
    pub session_timeout_minutes: u32,
    pub allowed_email_domains: Vec<String>,
    pub maintenance_mode: bool,
    pub maintenance_message: Option<String>,
    pub maintenance_until: Option<String>,
    pub maintenance_force_logout_at: Option<String>,
    pub maintenance_dismiss_seconds: i32,
    pub maintenance_warn_minutes: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct MaintenanceStatusResponse {
    pub active: bool,
    pub message: Option<String>,
    pub force_logout_at: Option<String>,
    pub until: Option<String>,
    pub dismiss_seconds: i32,
    pub warn_minutes: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct TerminateSessionsResponse {
    pub terminated_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct OverviewResponse {
    pub total_users: u64,
    pub active_sessions: u64,
    pub active_api_keys: u64,
    pub feature_flags_enabled: u64,
    pub feature_flags_total: u64,
}
