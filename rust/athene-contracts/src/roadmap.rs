//! Roadmap contracts (W7-R): the timeline of a workspace or one project.
//! The TypeScript in `src/generated/roadmap/` is generated from them (see
//! `tests/ts_bindings.rs`).
//!
//! - `GET /work/workspaces/:id/roadmap`: the planning items of the
//!   workspace's projects that take work (archived ones are left out),
//!   their `blocks` links, the projects' sprints and dated releases
//!   ([`RoadmapResponse`]).
//! - `?project_id=`: only that project (an archived one too).
//! - `?parent_id=`: the direct subtasks of that ticket instead of the
//!   planning items, each with its own child counts; no sprints or
//!   releases. A parent outside the workspace (or the named project)
//!   answers an empty list, never another tenant's tickets.
//!
//! A **planning item** is a top-level ticket (no parent) that is an epic,
//! has subtasks, or has a planned start. A bar runs from `start_date` to
//! `due_date` (else `effective_due`, the sprint's end); moving one is the
//! ordinary ticket edit, `PATCH /work/tickets/:id` with `start_date` and
//! `due_date` and their base in `expected` (409 `ATH-RES-003` on a stale
//! base, as for every field).
//!
//! The answer is wrapped as `{ "data": … }` and read in one statement.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::listing::EffectiveDue;
use crate::work::{MilestoneStatus, SprintStatus, TicketPriority};
use crate::workflow::StatusCategory;

/// At most this many items in one answer; `truncated` says when there
/// were more (the ones with the earliest dates come first).
pub const ROADMAP_MAX_ITEMS: usize = 2000;

/// Query of `GET /work/workspaces/:id/roadmap`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "roadmap/", optional_fields)]
pub struct RoadmapQuery {
    /// Only this project of the workspace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// The direct subtasks of this ticket instead of the planning items.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

/// How an item's subtasks stand, by what their statuses mean. `open` is
/// `total` minus the others.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "roadmap/")]
pub struct RoadmapChildCounts {
    pub total: u32,
    pub done: u32,
    pub in_progress: u32,
    pub blocked: u32,
}

/// One row of the timeline: a ticket with its dates and its subtasks'
/// progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "roadmap/")]
pub struct RoadmapItem {
    pub id: String,
    pub project_id: String,
    /// With the project's key, the ticket's key (`WEB-12`).
    pub sequence: i64,
    pub title: String,
    pub ticket_type: String,
    /// A status key of the workspace.
    pub status: String,
    #[schema(value_type = String)]
    pub status_category: StatusCategory,
    #[schema(value_type = String)]
    pub priority: TicketPriority,
    pub assignee_id: Option<String>,
    pub parent_ticket_id: Option<String>,
    pub sprint_id: Option<String>,
    pub milestone_id: Option<String>,
    /// RFC 3339, as on the ticket.
    pub start_date: Option<String>,
    /// RFC 3339: the ticket's own due date.
    pub due_date: Option<String>,
    /// Its own due date, else its running or planned sprint's end.
    #[schema(value_type = Option<Object>)]
    pub effective_due: Option<EffectiveDue>,
    pub children: RoadmapChildCounts,
    /// The ticket's write counter: orders answers, decides no conflict.
    pub version: i64,
    pub updated_at: String,
}

/// `from` blocks `to`; both are items of the same answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "roadmap/")]
pub struct RoadmapLink {
    pub from_ticket_id: String,
    pub to_ticket_id: String,
}

/// A sprint of one of the projects, on its days.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "roadmap/")]
pub struct RoadmapSprint {
    pub id: String,
    pub project_id: String,
    pub name: String,
    /// `YYYY-MM-DD`
    pub start_date: String,
    /// `YYYY-MM-DD`
    pub end_date: String,
    #[schema(value_type = String)]
    pub status: SprintStatus,
}

/// A release ("version") of one of the projects, on its date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "roadmap/")]
pub struct RoadmapRelease {
    pub id: String,
    pub project_id: String,
    pub title: String,
    /// `YYYY-MM-DD`
    pub due_date: String,
    #[schema(value_type = String)]
    pub status: MilestoneStatus,
}

/// `GET /work/workspaces/:id/roadmap`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "roadmap/")]
pub struct RoadmapResponse {
    /// By project, then by start (else due date), then by key.
    pub items: Vec<RoadmapItem>,
    /// More items than [`ROADMAP_MAX_ITEMS`]; the first are answered.
    pub truncated: bool,
    pub links: Vec<RoadmapLink>,
    /// By start. Empty for a `parent_id` read.
    pub sprints: Vec<RoadmapSprint>,
    /// Releases with a date, by date. Empty for a `parent_id` read.
    pub releases: Vec<RoadmapRelease>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_item_reads_as_the_wire_has_it() {
        let item: RoadmapItem = serde_json::from_value(serde_json::json!({
            "id": "t", "project_id": "p", "sequence": 3, "title": "Checkout",
            "ticket_type": "epic", "status": "in_progress", "status_category": "in_progress",
            "priority": "high", "assignee_id": null, "parent_ticket_id": null,
            "sprint_id": null, "milestone_id": null,
            "start_date": "2026-11-02T00:00:00+00:00", "due_date": null,
            "effective_due": null,
            "children": { "total": 4, "done": 1, "in_progress": 2, "blocked": 0 },
            "version": 7, "updated_at": "2026-10-28T09:00:00+00:00"
        }))
        .expect("item");
        assert_eq!(item.children.total, 4);
        assert_eq!(item.status_category, StatusCategory::InProgress);
        let query: RoadmapQuery =
            serde_json::from_value(serde_json::json!({ "project_id": "p" })).expect("query");
        assert_eq!(query.parent_id, None);
    }
}
