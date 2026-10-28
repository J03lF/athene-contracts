//! A person's own account: deleting it and taking their data along (W9,
//! C17).
//!
//! - `GET /user/account/deletion-check` ([`AccountDeletionCheck`]): what a
//!   deletion would do and what blocks it.
//! - `POST /user/account/delete` ([`DeleteAccountRequest`] →
//!   [`DeleteAccountResponse`]): with the password. Wrong ⇒ `401`
//!   [`ACCOUNT_WRONG_PASSWORD_CODE`]; blocked ⇒ `409`
//!   [`ACCOUNT_BLOCKED_BY_WORKSPACES_CODE`] or
//!   [`ACCOUNT_LAST_ADMIN_CODE`]. The person's own data goes, their
//!   contributions to shared workspaces stay under „Gelöschte Person“, and
//!   every session ends.
//! - `GET /user/account/export` ([`AccountExport`]): the person's data as
//!   one JSON file (a download), bounded in size ([`EXPORT_MAX_BYTES`]).

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use validator::Validate;

use crate::user::{UserProfileResponse, UserSettingsResponse};

/// Deletion blocked: the person is the last owner of workspaces others
/// still use (`409`).
pub const ACCOUNT_BLOCKED_BY_WORKSPACES_CODE: &str = "ATH-ACCOUNT-001";
/// Deletion blocked: the person is the only active platform admin (`409`).
pub const ACCOUNT_LAST_ADMIN_CODE: &str = "ATH-ACCOUNT-002";
/// The password given for the deletion is wrong (`401`; the session stays
/// valid, the client must not sign out on it).
pub const ACCOUNT_WRONG_PASSWORD_CODE: &str = "ATH-ACCOUNT-003";

/// The name a deleted person's contributions show.
pub const DELETED_PERSON_NAME: &str = "Gelöschte Person";

/// `format` of an export.
pub const EXPORT_FORMAT: &str = "athenos-account-export";
/// `version` of the export's shape.
pub const EXPORT_VERSION: u32 = 1;
/// The text an export carries at most (descriptions, comments, notes);
/// past it the lists stop and `truncated` is set.
pub const EXPORT_MAX_BYTES: usize = 8 * 1024 * 1024;
/// Entries per list at most.
pub const EXPORT_MAX_ROWS: usize = 5_000;

/// A workspace as the deletion check names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "account/")]
pub struct WorkspaceRef {
    pub id: String,
    pub name: String,
    pub member_count: u32,
}

/// `GET /user/account/deletion-check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "account/")]
pub struct AccountDeletionCheck {
    /// The person is the last owner, and others are members: ownership
    /// has to be handed over first.
    pub blocking_workspaces: Vec<WorkspaceRef>,
    /// The person is the only member: deleted with the account.
    pub deleted_with_account: Vec<WorkspaceRef>,
    /// The person is the only active platform admin.
    pub last_platform_admin: bool,
}

impl AccountDeletionCheck {
    /// Nothing stands in the way.
    pub fn allowed(&self) -> bool {
        self.blocking_workspaces.is_empty() && !self.last_platform_admin
    }
}

/// `POST /user/account/delete`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "account/")]
pub struct DeleteAccountRequest {
    #[validate(length(min = 1, max = 1024))]
    pub password: String,
}

impl std::fmt::Debug for DeleteAccountRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeleteAccountRequest")
            .field("password", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "account/")]
pub struct DeleteAccountResponse {
    pub deleted: bool,
}

/// One accepted version of the legal texts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "account/")]
pub struct ExportedConsent {
    pub version: String,
    pub accepted_at: String,
}

/// A workspace the person belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "account/")]
pub struct ExportedWorkspace {
    pub id: String,
    pub name: String,
    /// `owner`, `admin`, `member` or `guest`.
    pub role: String,
    pub joined_at: String,
}

/// A ticket the person reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "account/")]
pub struct ExportedTicket {
    pub id: String,
    pub project_id: String,
    pub key: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
}

/// A comment the person wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "account/")]
pub struct ExportedComment {
    pub id: String,
    pub ticket_id: String,
    pub body: String,
    pub created_at: String,
}

/// Time the person logged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "account/")]
pub struct ExportedWorkLog {
    pub id: String,
    pub ticket_id: String,
    pub minutes: i64,
    pub note: Option<String>,
    pub logged_at: String,
}

/// `GET /user/account/export`: the file
/// `athenos-export-YYYY-MM-DD.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "account/")]
pub struct AccountExport {
    /// [`EXPORT_FORMAT`].
    pub format: String,
    /// [`EXPORT_VERSION`].
    pub version: u32,
    pub exported_at: String,
    pub profile: UserProfileResponse,
    /// `null` when the person never changed a setting.
    pub settings: Option<UserSettingsResponse>,
    pub legal_consents: Vec<ExportedConsent>,
    pub workspaces: Vec<ExportedWorkspace>,
    pub tickets: Vec<ExportedTicket>,
    pub comments: Vec<ExportedComment>,
    pub work_logs: Vec<ExportedWorkLog>,
    /// A list stopped at [`EXPORT_MAX_ROWS`] or [`EXPORT_MAX_BYTES`].
    pub truncated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_password_never_prints() {
        let req: DeleteAccountRequest =
            serde_json::from_value(json!({ "password": "hunter2-hunter2" })).unwrap();
        assert!(!format!("{req:?}").contains("hunter2"));
        assert!(serde_json::from_value::<DeleteAccountRequest>(
            json!({ "password": "x", "now": true })
        )
        .is_err());
    }

    #[test]
    fn a_check_without_blockers_allows() {
        let mut check = AccountDeletionCheck {
            blocking_workspaces: Vec::new(),
            deleted_with_account: vec![WorkspaceRef {
                id: "w".into(),
                name: "Solo".into(),
                member_count: 1,
            }],
            last_platform_admin: false,
        };
        assert!(check.allowed());
        check.last_platform_admin = true;
        assert!(!check.allowed());
    }
}
