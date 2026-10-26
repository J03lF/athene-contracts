//! Paged ticket lists (step 3.9, `phase5/PERF.md` "Entwurf für 3.9"):
//! [`Page`], the keyset cursor's vocabulary, the [`TicketSummary`]
//! projection and a ticket's [`EffectiveDue`].
//!
//! `GET /work/tickets` answers a [`Page`] as soon as the query names any of
//! the list parameters of [`crate::work::ListTicketsQuery`] beyond
//! `project_id` and `workspace_id`. Without them it answers the whole list
//! as before, `{ "data": [...] }` (one release, for the readers that still
//! hold it, QA-07).
//!
//! The cursor is opaque: a client sends back what `meta.next_cursor` said,
//! with the same filters and sort. A cursor of another query is a `400`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::work::{TicketPriority, TicketStatus};

/// Tickets per page when the query names none.
pub const TICKET_PAGE_DEFAULT_LIMIT: u32 = 50;
/// The most tickets one page holds; a larger `limit` is clamped.
pub const TICKET_PAGE_MAX_LIMIT: u32 = 200;
/// The longest cursor the server hands out or reads. A title sort carries
/// the title (up to 160 characters, 640 bytes in UTF-8) in base64url.
pub const TICKET_CURSOR_MAX_CHARS: usize = 2048;
/// The longest list search (`q`).
pub const TICKET_LIST_QUERY_MAX_CHARS: usize = 100;
/// The most values one comma-separated filter takes.
pub const TICKET_LIST_MAX_FILTER_VALUES: usize = 50;

/// One page of a list. `data` is in the order the query asked for.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "work/")]
pub struct Page<T> {
    pub data: Vec<T>,
    pub meta: PageMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct PageMeta {
    /// The cursor of the next page; `null` on the last one.
    pub next_cursor: Option<String>,
    /// The page size the server applied (after clamping).
    pub limit: u32,
    /// How many tickets match the filters in all, with `count=true` on a
    /// paged query (one more statement); absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub total: Option<u64>,
}

/// The order of a paged ticket list. Empty values (no due date, no
/// estimate, no rank) sort last in both directions; ties fall back to the
/// ticket's id, so the order is total and a page boundary never splits or
/// repeats a ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "work/")]
pub enum TicketSort {
    /// Last change (`updated_at`), the default.
    #[default]
    Updated,
    /// Creation (`created_at`).
    Created,
    /// The effective due date ([`EffectiveDue`]).
    Due,
    /// Critical, high, medium, low.
    Priority,
    /// The workflow: open, in progress, in review, blocked, done (closed
    /// counts as done).
    Status,
    /// The title, ASCII letters case-insensitive.
    Title,
    /// The ticket key: the project's key, then the number.
    Key,
    /// The estimate (`story_points`).
    Points,
    /// A board column's order (`board_rank`); unranked tickets after the
    /// ranked ones, newest first.
    Rank,
}

/// The direction of [`TicketSort`]. Absent: the sort's own default —
/// descending for `updated`, `created`, `priority` and `points`, ascending
/// for the others.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "work/")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// How much of each ticket a list carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "work/")]
pub enum TicketFields {
    /// [`TicketSummary`]: no description, custom values or triage reason
    /// (about 60 % less). The default of a paged list.
    #[default]
    Summary,
    /// The whole [`crate::work::TicketResponse`].
    Full,
}

/// Where a ticket's deadline comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "work/")]
pub enum DueSource {
    /// The ticket's own `due_date`.
    Own,
    /// The end of the sprint it is in (a sprint that is not completed),
    /// as midday UTC of that day.
    Sprint,
}

/// A ticket's effective due date (open item §3.4 of W3C, BE-21): its own
/// due date, else the end of its sprint while that sprint is not
/// completed, else none. The server computes it in every ticket read; a
/// client shows it and never derives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "work/")]
pub struct EffectiveDue {
    /// RFC 3339.
    pub at: String,
    pub source: DueSource,
}

/// A ticket in a list (`fields=summary`): [`crate::work::TicketResponse`]
/// without `description`, `custom_values` and `triage_reason`. Open the
/// ticket (`GET /work/tickets/{id}`) for those.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "work/")]
pub struct TicketSummary {
    pub id: String,
    pub project_id: String,
    pub sequence: i64,
    pub title: String,
    pub ticket_type: String,
    pub status: TicketStatus,
    pub priority: TicketPriority,
    pub assignee_id: Option<String>,
    pub reporter_id: String,
    pub sla_policy_id: Option<String>,
    pub parent_ticket_id: Option<String>,
    pub sprint_id: Option<String>,
    pub milestone_id: Option<String>,
    pub team_id: Option<String>,
    pub adhoc_field_ids: Vec<String>,
    pub story_points: Option<i32>,
    pub triage_category: Option<String>,
    pub triaged_at: Option<String>,
    pub first_response_at: Option<String>,
    pub resolved_at: Option<String>,
    pub due_date: Option<String>,
    pub effective_due: Option<EffectiveDue>,
    pub sla_breached: bool,
    pub board_rank: Option<i64>,
    pub handoff_from: Option<String>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// `GET /work/workspaces/{workspace_id}/sprints` (Q-029): the sprints of
/// every project of the workspace in one answer, instead of one request
/// per project. A non-member gets `403`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/", optional_fields)]
pub struct WorkspaceSprintsQuery {
    /// Only these statuses (`upcoming,active,completed`); all without.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vocabulary_is_snake_case_on_the_wire() {
        assert_eq!(
            serde_json::to_value(TicketSort::Updated).unwrap(),
            serde_json::json!("updated")
        );
        assert_eq!(TicketSort::default(), TicketSort::Updated);
        assert_eq!(TicketFields::default(), TicketFields::Summary);
        assert_eq!(
            serde_json::to_value(DueSource::Sprint).unwrap(),
            serde_json::json!("sprint")
        );
        let parsed: SortOrder = serde_json::from_value(serde_json::json!("asc")).unwrap();
        assert_eq!(parsed, SortOrder::Asc);
    }

    #[test]
    fn a_page_without_a_count_leaves_total_out() {
        let page = Page::<u8> {
            data: vec![1],
            meta: PageMeta {
                next_cursor: None,
                limit: 50,
                total: None,
            },
        };
        let json = serde_json::to_value(&page).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "data": [1], "meta": { "next_cursor": null, "limit": 50 } })
        );
    }
}
