//! Admin contracts: feature flags, API keys, settings, maintenance, sessions,
//! the audit log and the overview.
//!
//! These are the wire types of `/admin/*` (and `/maintenance/status`). The
//! gateway validates and forwards request bodies with them, the core reads
//! requests and builds responses with them, and the TypeScript in
//! `src/generated/admin/` is generated from them (see `tests/ts_bindings.rs`).
//!
//! Request types reject unknown fields: a field the contract does not know is
//! a 400 at the edge instead of being dropped on the way to the core.
//! Timestamps in responses are RFC 3339 strings; in requests they are parsed,
//! so a malformed one fails at the edge too.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use validator::Validate;

use crate::auth::RegistrationMode;

// ---------------------------------------------------------------------------
// Feature flags
// ---------------------------------------------------------------------------

/// Create a feature flag (`POST /admin/feature-flags`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "admin/")]
pub struct FeatureFlagRequest {
    #[validate(length(min = 1, max = 128))]
    pub key: String,
    #[validate(length(min = 1, max = 512))]
    pub description: String,
    /// Defaults to `false`.
    #[serde(default)]
    #[ts(optional, as = "Option<bool>")]
    pub enabled: bool,
    /// Defaults to `false`.
    #[serde(default)]
    #[ts(optional, as = "Option<bool>")]
    pub requires_api_key: bool,
}

/// Change a feature flag (`PATCH /admin/feature-flags/:key`). Absent fields
/// stay as they are; the key itself is the path and cannot change.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "admin/", optional_fields)]
pub struct UpdateFeatureFlagRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1, max = 512))]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_api_key: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// Switch a feature flag on or off (`POST /admin/feature-flags/:key/toggle`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "admin/")]
pub struct ToggleRequest {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "admin/")]
pub struct FeatureFlagResponse {
    pub key: String,
    pub description: String,
    pub enabled: bool,
    pub requires_api_key: bool,
    pub created_at: String,
    pub updated_at: String,
    /// Id of the admin who last changed the flag.
    pub updated_by: Option<String>,
}

// ---------------------------------------------------------------------------
// API keys
// ---------------------------------------------------------------------------

/// Create an API key (`POST /admin/api-keys`).
///
/// The core stores an absolute expiry, so the request carries one:
/// `expires_at` is an RFC 3339 instant in the future, absent for a key that
/// never expires.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "admin/")]
pub struct CreateApiKeyRequest {
    #[validate(length(min = 1, max = 128))]
    pub name: String,
    /// Defaults to no scopes. Blank entries are dropped by the core.
    #[serde(default)]
    #[ts(optional, as = "Option<Vec<String>>")]
    pub scopes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<String>")]
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "admin/")]
pub struct ApiKeyResponse {
    pub id: String,
    pub name: String,
    /// The first characters of the key, for recognising it in a list.
    pub key_prefix: String,
    /// `active`, `revoked` or `expired`.
    pub status: String,
    pub scopes: Vec<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
}

/// The created key and its secret. The secret is shown once and never again.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "admin/")]
pub struct ApiKeyCreatedResponse {
    pub api_key: ApiKeyResponse,
    pub key: String,
}

/// Answer to `DELETE /admin/api-keys/:id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "admin/")]
pub struct ApiKeyRevokedResponse {
    pub revoked: bool,
}

// ---------------------------------------------------------------------------
// Settings and maintenance
// ---------------------------------------------------------------------------

/// Switch maintenance mode (`PUT /admin/settings/maintenance`). When
/// `enabled` is false, the message and both instants are cleared.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "admin/", optional_fields = nullable)]
pub struct MaintenanceRequest {
    pub enabled: bool,
    #[serde(default)]
    #[validate(length(max = 500))]
    pub message: Option<String>,
    #[serde(default)]
    #[ts(as = "Option<String>")]
    pub until: Option<DateTime<Utc>>,
    #[serde(default)]
    #[ts(as = "Option<String>")]
    pub force_logout_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub dismiss_seconds: Option<i32>,
    #[serde(default)]
    pub warn_minutes: Option<i32>,
}

/// Change instance settings (`PUT /admin/settings`). Absent (or `null`)
/// fields stay as they are. Maintenance mode itself is switched through
/// [`MaintenanceRequest`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "admin/", optional_fields = nullable)]
pub struct UpdateSettingsRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_mode: Option<RegistrationMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_email_domains: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_users: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_teams: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 500))]
    pub maintenance_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<String>")]
    pub maintenance_until: Option<DateTime<Utc>>,
    /// Absolute session lifetime in days, `1..=90`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1, max = 90))]
    pub session_max_days: Option<u32>,
    /// Days without use before a session ends, `1..=90` and at most
    /// `session_max_days` (the stored one when that is absent): else `400`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1, max = 90))]
    pub session_idle_days: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beta_banner_enabled: Option<bool>,
    /// `^[A-Za-z0-9._-]{1,32}$`; a new value asks everyone to consent again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1, max = 32))]
    pub legal_version: Option<String>,
    /// At most 200 characters; `""` clears it (the pages show
    /// `[HOSTER, LAND]`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 200))]
    pub legal_hoster: Option<String>,
    /// `YYYY-MM-DD`; `""` clears it (the pages show `[DATUM]`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 10))]
    pub legal_effective_date: Option<String>,
    /// Workspaces a person may own; `0` = unlimited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota_workspaces_per_account: Option<u32>,
    /// Attachment megabytes over all workspaces a person owns; `0` =
    /// unlimited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota_storage_mb_per_account: Option<u32>,
    /// Tickets a person creates per rolling 24 hours; `0` = unlimited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota_tickets_per_day: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "admin/")]
pub struct SettingsResponse {
    pub registration_mode: RegistrationMode,
    pub maintenance_mode: bool,
    pub maintenance_message: Option<String>,
    pub maintenance_until: Option<String>,
    pub maintenance_force_logout_at: Option<String>,
    pub maintenance_dismiss_seconds: i32,
    pub maintenance_warn_minutes: i32,
    pub allowed_email_domains: Vec<String>,
    pub max_users: u32,
    pub max_teams: u32,
    pub updated_at: String,
    /// Id of the admin who last changed the settings.
    pub updated_by: Option<String>,
    /// Absolute session lifetime in days (auth-service reads it).
    pub session_max_days: u32,
    /// Days without use before a session ends.
    pub session_idle_days: u32,
    pub beta_banner_enabled: bool,
    /// The legal texts' version people consent to.
    pub legal_version: String,
    pub legal_hoster: Option<String>,
    /// `YYYY-MM-DD`.
    pub legal_effective_date: Option<String>,
    /// `0` = unlimited.
    pub quota_workspaces_per_account: u32,
    /// `0` = unlimited.
    pub quota_storage_mb_per_account: u32,
    /// `0` = unlimited.
    pub quota_tickets_per_day: u32,
}

/// Public maintenance status (`GET /maintenance/status` and its stream).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "admin/")]
pub struct MaintenanceStatusResponse {
    pub active: bool,
    pub message: Option<String>,
    pub force_logout_at: Option<String>,
    pub until: Option<String>,
    pub dismiss_seconds: i32,
    pub warn_minutes: i32,
}

// ---------------------------------------------------------------------------
// Sessions, audit log, caches
// ---------------------------------------------------------------------------

/// Answer to `POST /admin/sessions/terminate-all`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "admin/")]
pub struct TerminateSessionsResponse {
    pub terminated_count: u64,
}

/// One entry of `GET /admin/audit-log`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "admin/")]
pub struct AuditEntryResponse {
    pub id: String,
    pub action: String,
    /// The actor's email, else their id, else `system`.
    pub actor: String,
    /// `resource_type:resource_id`, or the type alone.
    pub target: Option<String>,
    pub detail: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: String,
}

/// Answer to the core's `POST /admin/cache/refresh` (not routed by the
/// gateway).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "admin/")]
pub struct CacheRefreshResponse {
    pub settings_refreshed: bool,
    pub feature_flags_refreshed: bool,
    pub sla_updates: u64,
}

// ---------------------------------------------------------------------------
// Service announcements
// ---------------------------------------------------------------------------

/// Real announcement sends allowed per rolling 24 hours (dry runs are free).
pub const ANNOUNCEMENT_SENDS_PER_DAY: u32 = 3;
/// The error code once they are used up (`429`).
pub const ANNOUNCEMENT_RATE_CODE: &str = "ATH-RATE-ANNOUNCE";

/// `POST /admin/announcements/mail`: a service mail (a planned reset, the
/// end of the beta) to **every active user**, whatever their notification
/// settings. `dry_run` only counts the recipients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "admin/")]
pub struct AnnouncementMailRequest {
    #[validate(length(min = 3, max = 150))]
    pub subject: String,
    /// Plain text; paragraphs separated by blank lines.
    #[validate(length(min = 10, max = 5000))]
    pub message: String,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "admin/")]
pub struct AnnouncementMailResponse {
    /// Active users the mail goes (or would go) to.
    pub recipients: u32,
    /// Mails handed to the mail service (`0` on a dry run).
    pub queued: u32,
    pub dry_run: bool,
}

// ---------------------------------------------------------------------------
// Overview
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "admin/")]
pub struct OverviewCounts {
    pub users: u64,
    pub feature_flags: u64,
    pub api_keys: u64,
    /// Keys that are neither revoked nor expired.
    pub active_api_keys: u64,
}

/// `GET /admin/overview`: counts, the current settings and the five most
/// recently changed flags and created keys.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "admin/")]
pub struct OverviewResponse {
    pub counts: OverviewCounts,
    pub settings: SettingsResponse,
    pub recent_feature_flags: Vec<FeatureFlagResponse>,
    pub recent_api_keys: Vec<ApiKeyResponse>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn settings_update_carries_max_teams_and_skips_absent_fields() {
        let req: UpdateSettingsRequest =
            serde_json::from_value(json!({ "max_users": 50, "max_teams": 7 })).unwrap();
        assert_eq!(req.max_teams, Some(7));
        assert_eq!(
            serde_json::to_value(&req).unwrap(),
            json!({ "max_users": 50, "max_teams": 7 })
        );
    }

    #[test]
    fn requests_reject_fields_the_contract_does_not_know() {
        assert!(serde_json::from_value::<UpdateSettingsRequest>(
            json!({ "session_timeout_minutes": 30 })
        )
        .is_err());
        assert!(serde_json::from_value::<CreateApiKeyRequest>(
            json!({ "name": "ci", "expires_in": "30d" })
        )
        .is_err());
    }

    #[test]
    fn api_key_create_takes_an_absolute_expiry() {
        let req: CreateApiKeyRequest = serde_json::from_value(json!({
            "name": "ci",
            "scopes": ["read"],
            "expires_at": "2030-01-02T03:04:05Z"
        }))
        .unwrap();
        assert_eq!(req.scopes, vec!["read".to_string()]);
        assert_eq!(
            req.expires_at.map(|at| at.to_rfc3339()),
            Some("2030-01-02T03:04:05+00:00".to_string())
        );
        assert!(serde_json::from_value::<CreateApiKeyRequest>(
            json!({ "name": "ci", "expires_at": "in 30 days" })
        )
        .is_err());

        let never: CreateApiKeyRequest = serde_json::from_value(json!({ "name": "ci" })).unwrap();
        assert!(never.scopes.is_empty());
        assert_eq!(
            serde_json::to_value(&never).unwrap(),
            json!({ "name": "ci", "scopes": [] })
        );
    }
}
