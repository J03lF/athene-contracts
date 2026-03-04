use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct UserSettingsResponse {
    pub theme: String,
    pub language: String,
    pub shortcuts: serde_json::Value,
    pub notifications_enabled: bool,
    pub email_notifications: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct UpdateUserSettingsRequest {
    pub theme: Option<String>,
    pub language: Option<String>,
    pub shortcuts: Option<serde_json::Value>,
    pub notifications_enabled: Option<bool>,
    pub email_notifications: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct UserProfileResponse {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub avatar_url: Option<String>,
    pub last_login_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
}
