//! Queues and hand-offs (custom roles, phase C1; `artifacts/roles/
//! CONCEPT.md` §4, decision E6).
//!
//! A **queue** maps a ticket status to a target role: when a ticket enters
//! that status (a status change, a board drag, a bulk edit), the same
//! statement records who had it (`handoff_from`), unassigns it when the
//! queue says so (`unassign_on_enter`, the default), and notes the queue in
//! the status change's activity line (`metadata.queue`). Members of the
//! target role find it under "Meine Queues" and in their inbox, and
//! **take** it: an atomic compare-and-set on "nobody has it" — the first
//! one wins, everyone else gets `409` [`ALREADY_TAKEN_CODE`] naming the
//! winner. **Handing back** returns the ticket to `in_progress` and to the
//! person it came from, with a required comment.
//!
//! For C1 a queue targets a **role** ([`QueueTarget::Role`]); a team
//! target ("the ticket's team", once tickets carry one) is the hook for
//! later — the table already has the column. "Bereit zum Test" is the
//! existing `in_review` with a display name per workspace
//! ([`crate::workspace::StatusLabels`]); real custom statuses come in C2.
//!
//! - `GET /work/workspaces/:workspace_id/queues` — every member: the
//!   queues with how many tickets wait in each, whether the caller works
//!   it, and whether the caller may edit them.
//! - `POST /work/workspaces/:workspace_id/queues` — a new queue
//!   (`workspace.settings.edit`: owners and admins).
//! - `PATCH /work/workspaces/:workspace_id/queues/:queue_id` — field-level
//!   with `expected` (3.7: a field changed meanwhile is a `409`
//!   `ATH-RES-003`).
//! - `DELETE /work/workspaces/:workspace_id/queues/:queue_id`.
//! - `GET /work/workspaces/:workspace_id/queues/mine` — the queues the
//!   caller works, with the tickets waiting in them.
//! - `POST /work/tickets/:id/take` — `ticket.take`, and the caller works
//!   the ticket's queue.
//! - `POST /work/tickets/:id/hand-back` — `ticket.take`, `ticket.edit` and
//!   `comment.create`, and the caller works the ticket's queue.
//!
//! Request bodies refuse unknown fields; every response is `{ "data": … }`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::work::{TicketExpected, TicketResponse, TicketStatus};

/// `409`: somebody else took the ticket first. `details.current` is the
/// ticket now, `details.taken_by` `{ id, display_name }` who has it.
pub const ALREADY_TAKEN_CODE: &str = "ATH-RES-005";
/// The i18n key of [`ALREADY_TAKEN_CODE`].
pub const ALREADY_TAKEN_MESSAGE_KEY: &str = "errors.already_taken";
/// `409`: the ticket waits in no queue (any more): its status has no queue,
/// or it left the queue's status meanwhile. `details.current` is the
/// ticket now.
pub const NOT_IN_QUEUE_CODE: &str = "ATH-RES-006";
/// The i18n key of [`NOT_IN_QUEUE_CODE`].
pub const NOT_IN_QUEUE_MESSAGE_KEY: &str = "errors.not_in_queue";

/// The longest queue name, in characters (one line).
pub const QUEUE_MAX_NAME_CHARS: usize = 60;
/// The most queues a workspace may hold.
pub const QUEUE_MAX_PER_WORKSPACE: usize = 30;
/// The longest hand-back reason, in characters: a comment's limit.
pub const HAND_BACK_MAX_COMMENT_CHARS: usize = crate::limits::COMMENT_MAX_CHARS;
/// The most waiting tickets `queues/mine` lists per queue.
pub const QUEUE_MAX_LISTED_TICKETS: usize = 100;

/// Who works a queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "queues/")]
pub enum QueueTargetKind {
    /// The members holding one role of the workspace.
    Role,
}

/// A queue of a workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "queues/")]
pub struct QueueResponse {
    pub id: String,
    pub workspace_id: String,
    /// `null`: every project of the workspace. A project's own queue for a
    /// status comes before the workspace-wide one.
    pub project_id: Option<String>,
    pub name: String,
    /// The status a ticket waits in: `open`, `in_progress`, `in_review` or
    /// `blocked` (never an end state).
    #[schema(value_type = String)]
    pub status: TicketStatus,
    pub target_kind: QueueTargetKind,
    /// The target role's id.
    pub target_id: String,
    /// The target role's name and colour token, for the badge.
    pub target_name: String,
    pub target_color: Option<String>,
    /// Entering the status unassigns the ticket (the hand-off). Without it
    /// the ticket keeps its assignee and is only marked.
    pub unassign_on_enter: bool,
    /// Display order, lowest first.
    pub position: i32,
    /// Tickets in the status, unassigned, in a live project the queue
    /// covers.
    pub waiting: i64,
    /// The caller holds the target role.
    pub mine: bool,
    /// The caller may take from it: holds the target role, or governs the
    /// workspace (owners and admins), and has `ticket.take`.
    pub can_take: bool,
    /// For `PATCH` (3.7): orders answers, decides no conflict.
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// `GET /work/workspaces/:workspace_id/queues`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "queues/")]
pub struct WorkspaceQueuesResponse {
    pub queues: Vec<QueueResponse>,
    /// The caller may create, edit and delete queues
    /// (`workspace.settings.edit`).
    pub editable: bool,
}

/// `POST /work/workspaces/:workspace_id/queues`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "queues/")]
pub struct CreateQueueRequest {
    /// 1–60 characters, trimmed.
    pub name: String,
    /// See [`QueueResponse::status`]. One queue per status and place.
    #[schema(value_type = String)]
    pub status: TicketStatus,
    pub target_kind: QueueTargetKind,
    /// A role of the workspace.
    pub target_id: String,
    /// A project of the workspace; absent or `null` = workspace-wide.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub project_id: Option<String>,
    /// Absent = `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub unassign_on_enter: Option<bool>,
}

/// The base of a queue edit: per field, the value the client saw.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "queues/")]
pub struct QueueExpected {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    #[schema(value_type = Option<String>)]
    pub status: Option<TicketStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub target_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub unassign_on_enter: Option<bool>,
}

/// `PATCH /work/workspaces/:workspace_id/queues/:queue_id`: the fields
/// named change, each checked against its base in `expected` (required for
/// every field sent). The place (`project_id`) does not change; a queue
/// for another place is a new queue.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "queues/")]
pub struct UpdateQueueRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    #[schema(value_type = Option<String>)]
    pub status: Option<TicketStatus>,
    /// A role of the workspace (the target stays a role in C1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub target_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub unassign_on_enter: Option<bool>,
    pub expected: QueueExpected,
}

/// `DELETE …/queues/:queue_id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "queues/")]
pub struct DeleteQueueResponse {
    pub id: String,
    pub deleted: bool,
}

/// One queue the caller works, with what waits in it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "queues/")]
pub struct QueueTickets {
    pub queue: QueueResponse,
    /// Waiting tickets (status, unassigned, live project), most urgent
    /// first, at most [`QUEUE_MAX_LISTED_TICKETS`]; `queue.waiting` is the
    /// full count.
    pub tickets: Vec<TicketResponse>,
}

/// `GET /work/workspaces/:workspace_id/queues/mine`: the queues whose
/// target role the caller holds, then (for owners and admins) the others.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "queues/")]
pub struct MyQueuesResponse {
    pub queues: Vec<QueueTickets>,
}

/// `POST /work/tickets/:id/hand-back`: back to `in_progress` and to the
/// person it came from (`handoff_from`), with a comment saying why.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "queues/")]
pub struct HandBackTicketRequest {
    /// Why it goes back: a comment's length (`limits::COMMENT_MIN_CHARS`..=
    /// `COMMENT_MAX_CHARS`), trimmed. Stored as a comment.
    pub comment: String,
    /// The base: `status` and `assignee_id` as the client saw them (3.7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected: Option<TicketExpected>,
}

/// Who has a ticket that was taken first (`409` [`ALREADY_TAKEN_CODE`]).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "queues/")]
pub struct TakenBy {
    pub id: String,
    /// `null` when the account is gone or no longer a member.
    pub display_name: Option<String>,
}

/// The queue facts in the activity feed (`metadata`):
/// - `ticket_status_changed` with `queue`: the ticket entered this queue
///   ("an die Tester-Queue übergeben"); `handed_off_by` had it.
/// - `ticket_assigned` with `taken_from`: the assignee took it from this
///   queue ("aus der Tester-Queue genommen").
/// - `ticket_status_changed` with `handed_back` (`{ to, comment_id, queue }`):
///   it went back to `to` ("an Anna zurückgeschickt").
///
/// The existing verbs keep the inbox, the reports and the conflict hints
/// ("who changed the assignee") working as they are.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "queues/")]
pub struct QueueActivity {
    pub id: String,
    pub name: String,
    /// The target role.
    pub role_id: String,
    /// Who had the ticket before it entered the queue (`null`: nobody).
    #[serde(default)]
    #[ts(optional = nullable)]
    pub handed_off_by: Option<String>,
}

/// Statuses a queue may wait in: every status that is not an end.
pub const QUEUE_STATUSES: [TicketStatus; 4] = [
    TicketStatus::Open,
    TicketStatus::InProgress,
    TicketStatus::InReview,
    TicketStatus::Blocked,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_refuse_unknown_fields_and_take_their_defaults() {
        let body = serde_json::json!({
            "name": "Tester-Queue", "status": "in_review",
            "target_kind": "role", "target_id": "r"
        });
        let create: CreateQueueRequest = serde_json::from_value(body).unwrap();
        assert_eq!(create.project_id, None);
        assert_eq!(create.unassign_on_enter, None);
        assert!(
            serde_json::from_value::<CreateQueueRequest>(serde_json::json!({
                "name": "x", "status": "in_review", "target_kind": "team", "target_id": "t"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<UpdateQueueRequest>(serde_json::json!({
                "name": "x", "expected": { "name": "y" }, "colour": "red"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<HandBackTicketRequest>(serde_json::json!({
                "comment": "Fails on Safari", "assignee_id": "x"
            }))
            .is_err()
        );
    }

    #[test]
    fn no_queue_waits_in_an_end_state() {
        assert!(!QUEUE_STATUSES.contains(&TicketStatus::Done));
        assert!(!QUEUE_STATUSES.contains(&TicketStatus::Closed));
    }
}
