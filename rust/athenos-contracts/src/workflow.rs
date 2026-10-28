//! A workspace's own ticket statuses (W4-F, "Workflow"; migration 0042).
//!
//! Every workspace holds its statuses as rows: the five built-in ones
//! (`open`, `in_progress`, `in_review`, `blocked`, `done`, keys unchanged,
//! so every ticket stays valid) and the ones it adds. A status is a real
//! ticket status: a board column is a status, a queue hangs on one, a
//! ticket's `status` is its key.
//!
//! What a status *means* is its [`StatusCategory`]: `open`, `in_progress`,
//! `blocked` or `done`. Everything that depends on a status asks the
//! category — closing (`ticket.close`, the fields required at close), the
//! sprint and report counts, "open" work, the board's done window, and
//! which statuses a queue may wait in (never a `done` one).
//!
//! - The built-in statuses keep their key and their category; they can be
//!   renamed (one name, or one per language), recoloured, reordered and
//!   hidden per project, but not archived or deleted. `in_review` stays
//!   the default status of a new queue ("Bereit zum Test"); its name is
//!   also the workspace setting `status_labels.in_review` (W3-B), kept in
//!   step both ways.
//! - A workspace's own status gets a key (from its name when none is
//!   given) that never changes. Its category changes only while no ticket
//!   holds it. It can be archived (no new ticket enters it; tickets in it
//!   stay and show) and deleted when no ticket holds it — or with
//!   `move_to`, which moves its tickets in the same statement.
//! - Per project a status can be hidden: not offered there; a ticket
//!   already in it still shows. `open` cannot be hidden.
//! - Legacy `closed` values stay as stored and count as `done` (UX-023);
//!   a write never sets `closed` any more (`closed` as a target is `done`).
//!
//! Routes (every member reads; owners and admins edit:
//! `workspace.settings.edit`):
//! - `GET /work/workspaces/:workspace_id/statuses[?with_usage=true]`
//! - `POST /work/workspaces/:workspace_id/statuses`
//! - `PATCH /work/workspaces/:workspace_id/statuses/:status_id` —
//!   field-level with `expected` (3.7: `409 ATH-RES-003` on a field
//!   changed meanwhile).
//! - `PUT /work/workspaces/:workspace_id/statuses/order`
//! - `POST /work/workspaces/:workspace_id/statuses/:status_id/delete`
//! - `PUT /work/workspaces/:workspace_id/statuses/hidden` — the statuses
//!   one project of the workspace hides.
//!
//! Request bodies refuse unknown fields; every response is `{ "data": … }`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::Patch;

/// What a status means.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    ToSchema,
    TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "workflow/")]
pub enum StatusCategory {
    /// Not started.
    #[default]
    Open,
    /// Being worked on, reviewed or tested.
    InProgress,
    /// Waiting on something.
    Blocked,
    /// Finished: closing a ticket is moving it into a `done` status.
    Done,
}

impl StatusCategory {
    /// Every category, in board order.
    pub const ALL: [Self; 4] = [Self::Open, Self::InProgress, Self::Blocked, Self::Done];

    /// The category on the wire and in the database.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Done => "done",
        }
    }

    /// The category `value` names.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == value)
    }

    /// A ticket in a status of this category is finished.
    pub fn closes(self) -> bool {
        self == Self::Done
    }
}

/// One of the five statuses every workspace has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinStatus {
    pub key: &'static str,
    pub category: StatusCategory,
    /// Its place in a new workspace's order.
    pub position: i32,
    /// A colour token of [`STATUS_COLORS`].
    pub color: &'static str,
}

/// The built-in statuses, in their default order. Their keys are the
/// ticket statuses tickets have always carried.
pub const BUILTIN_STATUSES: [BuiltinStatus; 5] = [
    BuiltinStatus {
        key: "open",
        category: StatusCategory::Open,
        position: 0,
        color: "neutral",
    },
    BuiltinStatus {
        key: "in_progress",
        category: StatusCategory::InProgress,
        position: 1,
        color: "blue",
    },
    BuiltinStatus {
        key: "in_review",
        category: StatusCategory::InProgress,
        position: 2,
        color: "purple",
    },
    BuiltinStatus {
        key: "blocked",
        category: StatusCategory::Blocked,
        position: 3,
        color: "red",
    },
    BuiltinStatus {
        key: "done",
        category: StatusCategory::Done,
        position: 4,
        color: "green",
    },
];

/// The status a new ticket starts in; it cannot be hidden.
pub const START_STATUS: &str = "open";
/// The status a new queue suggests ("Bereit zum Test", custom roles C1).
pub const DEFAULT_QUEUE_STATUS: &str = "in_review";
/// Where "Zurückschicken" sends a ticket.
pub const HAND_BACK_STATUS: &str = "in_progress";
/// Older values a ticket may still hold: they count as `done`. Never
/// written any more; a write naming one sets `done`.
pub const LEGACY_DONE_KEYS: [&str; 2] = ["closed", "resolved"];
/// Spellings older clients send for built-in statuses.
pub const STATUS_ALIASES: [(&str, &str); 2] = [("prog", "in_progress"), ("review", "in_review")];

/// The longest key: `^[a-z][a-z0-9_]{0,31}$`.
pub const STATUS_KEY_MAX_CHARS: usize = 32;
/// The longest name, in characters (one line).
pub const STATUS_NAME_MAX_CHARS: usize = crate::workspace::STATUS_LABEL_MAX_CHARS;
/// The most statuses a workspace holds, archived ones included.
pub const WORKFLOW_MAX_STATUSES: usize = 40;
/// The colour tokens a status may use (the ticket types' tokens).
pub const STATUS_COLORS: &[&str] = crate::ticket_types::TICKET_TYPE_COLORS;

/// `409` reasons (`details.reason`) of a status edit.
pub const STATUS_IN_USE_REASON: &str = "status_in_use";
/// `409`: a queue waits in the status, so it cannot become `done`.
pub const STATUS_HAS_QUEUES_REASON: &str = "status_has_queues";
/// `400` reasons (`details.reason`) of a ticket write into a status.
pub const STATUS_UNKNOWN_REASON: &str = "status_unknown";
pub const STATUS_ARCHIVED_REASON: &str = "status_archived";
pub const STATUS_HIDDEN_REASON: &str = "status_hidden";

/// The built-in status with this key.
pub fn builtin(key: &str) -> Option<BuiltinStatus> {
    BUILTIN_STATUSES.into_iter().find(|s| s.key == key)
}

/// The category a key has by itself: a built-in one, or a legacy `done`.
/// `None` for a workspace's own status (its row says).
pub fn fixed_category(key: &str) -> Option<StatusCategory> {
    builtin(key).map(|s| s.category).or_else(|| {
        LEGACY_DONE_KEYS
            .contains(&key)
            .then_some(StatusCategory::Done)
    })
}

/// The shape of a key: `^[a-z][a-z0-9_]{0,31}$`.
pub fn valid_key_shape(key: &str) -> bool {
    let mut chars = key.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && key.len() <= STATUS_KEY_MAX_CHARS
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// A key a workspace may give its own status: the shape, and none of the
/// built-in, legacy or alias keys.
pub fn valid_custom_key(key: &str) -> bool {
    valid_key_shape(key)
        && fixed_category(key).is_none()
        && STATUS_ALIASES.iter().all(|(alias, _)| *alias != key)
}

/// A status of the workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "workflow/")]
pub struct WorkflowStatusResponse {
    pub id: String,
    pub workspace_id: String,
    /// The ticket status: `TicketResponse.status`. Never changes.
    pub key: String,
    /// The name in every language; `null` only for a built-in status the
    /// workspace did not rename (show the product's word for `key`).
    pub name: Option<String>,
    /// The German name, when it differs from `name`.
    pub name_de: Option<String>,
    /// The English name, when it differs from `name`.
    pub name_en: Option<String>,
    pub category: StatusCategory,
    /// Board and list order, lowest first.
    pub position: i32,
    /// A token of [`STATUS_COLORS`].
    pub color: String,
    /// One of the five every workspace has.
    pub builtin: bool,
    /// `null` while live. An archived status takes no new tickets.
    pub archived_at: Option<String>,
    /// The projects of the workspace that hide it.
    pub hidden_in_projects: Vec<String>,
    /// Tickets of the workspace in it (with `?with_usage=true`; `null`
    /// otherwise).
    #[serde(default)]
    #[ts(optional = nullable)]
    pub ticket_count: Option<i64>,
    /// For `PATCH` (3.7): orders answers, decides no conflict.
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// `GET /work/workspaces/:workspace_id/statuses` and the answer of the
/// order and hidden-status edits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "workflow/")]
pub struct WorkspaceWorkflowResponse {
    /// By position, archived ones included.
    pub statuses: Vec<WorkflowStatusResponse>,
    /// The caller may edit them (`workspace.settings.edit`).
    pub editable: bool,
}

/// `GET …/statuses?with_usage=true`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "workflow/", optional_fields)]
pub struct WorkflowQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_usage: Option<bool>,
}

/// `POST /work/workspaces/:workspace_id/statuses`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workflow/")]
pub struct CreateWorkflowStatusRequest {
    /// 1–40 characters on one line, trimmed.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name_de: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name_en: Option<String>,
    pub category: StatusCategory,
    /// [`valid_custom_key`]; absent = made from the name (numbered on a
    /// clash).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub key: Option<String>,
    /// Absent = `neutral`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub color: Option<String>,
}

/// The base of a status edit: per field, the value the client saw.
/// Nullable fields: absent = not sent, `null` = it was empty.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workflow/")]
pub struct WorkflowStatusExpected {
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub name: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub name_de: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub name_en: Patch<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub category: Option<StatusCategory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub archived: Option<bool>,
}

/// `PATCH /work/workspaces/:workspace_id/statuses/:status_id`: the fields
/// named change, each checked against its base in `expected` (required
/// for every field sent). The key never changes; the order has its own
/// route.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workflow/")]
pub struct UpdateWorkflowStatusRequest {
    /// `null` (built-in statuses only): back to the product's word.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub name: Patch<String>,
    /// `null`: the same as `name`.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub name_de: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub name_en: Patch<String>,
    /// A workspace's own status only, and only while no ticket holds it
    /// (`409` `status_in_use` otherwise); never `done` while a queue
    /// waits in it (`409` `status_has_queues`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub category: Option<StatusCategory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub color: Option<String>,
    /// A workspace's own status only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub archived: Option<bool>,
    pub expected: WorkflowStatusExpected,
}

/// `PUT /work/workspaces/:workspace_id/statuses/order`: every status id
/// of the workspace, in the new order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workflow/")]
pub struct ReorderWorkflowStatusesRequest {
    pub ordered_ids: Vec<String>,
}

/// `POST /work/workspaces/:workspace_id/statuses/:status_id/delete`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workflow/")]
pub struct DeleteWorkflowStatusRequest {
    /// The key of a live status of the workspace the status's tickets
    /// move to, in the same statement. Without it, a status some ticket
    /// holds is not deleted (`409` `status_in_use`, with `details.tickets`):
    /// archive it, or name where its tickets go.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub move_to: Option<String>,
}

/// `POST …/statuses/:status_id/delete`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "workflow/")]
pub struct DeleteWorkflowStatusResponse {
    pub id: String,
    pub key: String,
    /// Tickets moved to `move_to`.
    pub moved_tickets: i64,
    pub move_to: Option<String>,
    /// Queues that waited in the status, removed with it.
    pub removed_queues: i64,
}

/// `PUT /work/workspaces/:workspace_id/statuses/hidden`: the keys hidden
/// in one live project of the workspace, all of them (`[]` shows every
/// status). `open` cannot be hidden.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workflow/")]
pub struct SetHiddenStatusesRequest {
    pub project_id: String,
    pub keys: Vec<String>,
}

/// `PUT …/statuses/hidden`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "workflow/")]
pub struct ProjectHiddenStatusesResponse {
    pub project_id: String,
    /// The keys hidden there, in workflow order.
    pub hidden: Vec<String>,
}

/// The facts of a status's activity lines (`workflow_status_*`):
/// `metadata = { key, category, … }`; an update also carries `fields`
/// (what changed), a hide `{ project_id, hidden }`, a delete
/// `{ moved_tickets, move_to, removed_queues }`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "workflow/")]
pub struct WorkflowStatusActivity {
    pub key: String,
    pub category: StatusCategory,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_statuses_keep_their_keys_and_categories() {
        let keys: Vec<&str> = BUILTIN_STATUSES.iter().map(|s| s.key).collect();
        assert_eq!(
            keys,
            ["open", "in_progress", "in_review", "blocked", "done"]
        );
        assert_eq!(
            fixed_category("in_review"),
            Some(StatusCategory::InProgress)
        );
        assert_eq!(fixed_category("closed"), Some(StatusCategory::Done));
        assert_eq!(fixed_category("resolved"), Some(StatusCategory::Done));
        assert_eq!(fixed_category("qa"), None);
        for status in BUILTIN_STATUSES {
            assert!(STATUS_COLORS.contains(&status.color), "{}", status.key);
            assert!(valid_key_shape(status.key));
            assert!(!valid_custom_key(status.key));
        }
        assert_eq!(
            builtin(DEFAULT_QUEUE_STATUS).map(|s| s.key),
            Some("in_review")
        );
        assert!(builtin(START_STATUS).is_some() && builtin(HAND_BACK_STATUS).is_some());
    }

    #[test]
    fn keys_have_one_shape() {
        assert!(valid_custom_key("qa"));
        assert!(valid_custom_key("ready_for_test2"));
        assert!(valid_custom_key(&"a".repeat(32)));
        for bad in [
            "", "2qa", "QA", "q-a", "qä", "closed", "prog", "review", "done",
        ] {
            assert!(!valid_custom_key(bad), "{bad}");
        }
        assert!(!valid_custom_key(&"a".repeat(33)));
    }

    #[test]
    fn categories_spell_as_serde_does() {
        for category in StatusCategory::ALL {
            let wire = serde_json::to_value(category).unwrap();
            assert_eq!(wire, serde_json::json!(category.as_str()));
            assert_eq!(StatusCategory::parse(category.as_str()), Some(category));
        }
        assert!(StatusCategory::Done.closes());
        assert!(!StatusCategory::Blocked.closes());
    }

    #[test]
    fn requests_refuse_unknown_fields() {
        let create: CreateWorkflowStatusRequest = serde_json::from_value(serde_json::json!({
            "name": "QA", "category": "in_progress"
        }))
        .unwrap();
        assert_eq!(create.key, None);
        assert!(
            serde_json::from_value::<CreateWorkflowStatusRequest>(serde_json::json!({
                "name": "QA", "category": "review"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<UpdateWorkflowStatusRequest>(serde_json::json!({
                "name": "QA", "expected": { "name": "x" }, "key": "qa"
            }))
            .is_err()
        );
        let update: UpdateWorkflowStatusRequest = serde_json::from_value(serde_json::json!({
            "name": null, "expected": { "name": "Bereit zum Test" }
        }))
        .unwrap();
        assert!(update.name.is_null());
        assert!(
            serde_json::from_value::<DeleteWorkflowStatusRequest>(serde_json::json!({
                "move_to": "done", "force": true
            }))
            .is_err()
        );
    }
}
