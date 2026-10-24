//! Activity feed contracts.
//!
//! The activity feed is UI-facing (separate from the security audit log).
//! See `ActivityRepository` in the athene crate for the persistence layer.
//! The TypeScript in `src/generated/activity/` is generated from these (see
//! `tests/ts_bindings.rs`).

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use ts_rs::TS;
use utoipa::ToSchema;

/// One feed entry: `GET /work/workspaces/:id/activity`,
/// `GET /work/tickets/:id/activity` and the home page's recent activity.
///
/// `detail` carries only user-authored text (a ticket title). Anything
/// machine-derived travels in `metadata`, so the client renders it in the
/// reader's language instead of printing a server-composed sentence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema, TS)]
#[ts(export_to = "activity/")]
pub struct ActivityEventResponse {
    pub id: String,
    pub workspace_id: String,
    /// `null` for system or automation events with no human actor.
    pub actor_id: Option<String>,
    /// Verb (e.g., `ticket_created`, `ticket_status_changed`,
    /// `sprint_started`). An open vocabulary: render an unknown verb as a
    /// generic entry (`ActivityAction` in the TypeScript names the known
    /// ones).
    pub action: String,
    /// `ticket`, `project`, `sprint`, `milestone`, ...
    pub target_type: String,
    pub target_id: String,
    /// Short user-authored text (e.g. a ticket title).
    pub detail: Option<String>,
    /// Structured payload for the verb. `ticket_status_changed` carries
    /// `{ from, to }` in the ticket-status vocabulary; `ticket_assigned`
    /// carries `{ assignee_id }`; `ticket_updated` carries
    /// `{ fields: string[] }`; `ticket_linked` carries
    /// `{ relation, to_ticket_id }`.
    #[ts(type = "Record<string, unknown> | null")]
    pub metadata: Option<JsonValue>,
    /// ISO-8601 (RFC 3339).
    pub created_at: String,
}

pub type ActivityListResponse = crate::ListResponse<ActivityEventResponse>;
