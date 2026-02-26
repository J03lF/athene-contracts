use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::common::{ApiResponse, ListResponse};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema)]
pub struct LoginConfirmRequest {
    #[validate(length(min = 1))]
    pub challenge_id: String,
    #[validate(length(min = 4, max = 12))]
    pub pin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 2, max = 100))]
    pub display_name: String,
    #[validate(length(min = 8))]
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema)]
pub struct RegisterConfirmRequest {
    #[validate(length(min = 1))]
    pub challenge_id: String,
    #[validate(length(min = 4, max = 12))]
    pub pin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema)]
pub struct PasswordResetRequest {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema)]
pub struct PasswordResetConfirmRequest {
    #[validate(length(min = 1))]
    pub token: String,
    #[validate(length(min = 8, max = 128))]
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct AuthChallenge {
    pub challenge_id: String,
    pub expires_at: DateTime<Utc>,
}

impl AuthChallenge {
    pub fn new(challenge_id: Uuid, expires_at: DateTime<Utc>) -> Self {
        Self {
            challenge_id: challenge_id.to_string(),
            expires_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub role: UserRole,
    pub status: UserStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub enum UserRole {
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "operator")]
    Operator,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "guest")]
    Guest,
}

impl Default for UserRole {
    fn default() -> Self {
        Self::User
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub enum UserStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "inactive")]
    Inactive,
    #[serde(rename = "locked")]
    Locked,
    #[serde(rename = "pending_verification")]
    PendingVerification,
}

impl Default for UserStatus {
    fn default() -> Self {
        Self::PendingVerification
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct AuthSession {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub user: AuthUser,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct RegistrationPolicy {
    pub registration_mode: RegistrationMode,
    pub allowed_email_domains: Vec<String>,
    pub max_users: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub enum RegistrationMode {
    #[serde(rename = "disabled")]
    Disabled,
    #[serde(rename = "api_key_required")]
    ApiKeyRequired,
    #[serde(rename = "open")]
    Open,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct SessionInfo {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub is_current: bool,
}

pub type AuthChallengeResponse = ApiResponse<AuthChallenge>;
pub type AuthSessionResponse = ApiResponse<AuthSession>;
pub type AuthUserResponse = ApiResponse<AuthUser>;
pub type RegistrationPolicyResponse = ApiResponse<RegistrationPolicy>;
pub type SessionListResponse = ListResponse<SessionInfo>;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct LogoutResponseData {
    pub revoked: bool,
}

pub type LogoutResponse = ApiResponse<LogoutResponseData>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct PasswordResetResponseData {
    pub requested: bool,
}

pub type PasswordResetResponse = ApiResponse<PasswordResetResponseData>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct PasswordConfirmResponseData {
    pub reset: bool,
}

pub type PasswordConfirmResponse = ApiResponse<PasswordConfirmResponseData>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct FeatureFlagState {
    pub key: String,
    pub enabled: bool,
}

pub type FeatureFlagListResponse = ListResponse<FeatureFlagState>;
