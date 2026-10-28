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
use utoipa::ToSchema;

pub use crate::search::FacetCount;
use crate::work::TicketPriority;

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
/// The most custom-field conditions one list takes (`cf`).
pub const TICKET_LIST_MAX_FIELD_FILTERS: usize = 10;
/// The longest `cf` parameter (its JSON).
pub const TICKET_LIST_FIELD_FILTERS_MAX_CHARS: usize = 4000;
/// The most custom fields a summary carries values of (`cf_keys`).
pub const TICKET_LIST_MAX_FIELD_KEYS: usize = 20;
/// The most ticket ids one `ids` filter takes (B4, W7-P): a feed or a
/// lookup names at most a page of tickets.
pub const TICKET_LIST_MAX_IDS: usize = 100;
/// The most values one facet answers (B6, W7-P), largest counts first;
/// the rest are left out (`TicketFacets` says how many matched in all
/// only through `PageMeta.total`).
pub const TICKET_FACET_MAX_VALUES: usize = 50;

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
    /// The counts per value of the facets `facets` asked for (B6, W7-P);
    /// absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub facets: Option<TicketFacets>,
}

/// A dimension a list can count its matches by (`facets=`, B6, W7-P).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "work/")]
pub enum TicketFacet {
    /// The status key (the legacy `closed` counts as `done`).
    Status,
    /// What the status means (`open`, `in_progress`, `blocked`, `done`).
    Category,
    /// The ticket-type key.
    Type,
    /// The assignee's id, `none` for unassigned.
    Assignee,
    /// `low`, `medium`, `high`, `critical`.
    Priority,
}

impl TicketFacet {
    pub const ALL: [TicketFacet; 5] = [
        TicketFacet::Status,
        TicketFacet::Category,
        TicketFacet::Type,
        TicketFacet::Assignee,
        TicketFacet::Priority,
    ];

    /// The facet's name on the wire.
    pub fn as_str(self) -> &'static str {
        match self {
            TicketFacet::Status => "status",
            TicketFacet::Category => "category",
            TicketFacet::Type => "type",
            TicketFacet::Assignee => "assignee",
            TicketFacet::Priority => "priority",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.as_str() == raw)
    }
}

/// The counts of a list per facet (B6, W7-P), in the search's
/// [`FacetCount`] (`value` `null`: unassigned). Each facet counts the
/// tickets that match every filter of the query **except its own**, over
/// all pages (the cursor plays no part), so a filter row can show what
/// each of its choices would find. Largest counts first, then by key; at
/// most [`TICKET_FACET_MAX_VALUES`] values each. A facet not asked for is
/// absent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "work/", optional_fields)]
pub struct TicketFacets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<Vec<FacetCount>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<Vec<FacetCount>>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub ticket_type: Option<Vec<FacetCount>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<Vec<FacetCount>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<Vec<FacetCount>>,
}

impl TicketFacets {
    /// The counts of `facet`, to fill in.
    pub fn slot(&mut self, facet: TicketFacet) -> &mut Option<Vec<FacetCount>> {
        match facet {
            TicketFacet::Status => &mut self.status,
            TicketFacet::Category => &mut self.category,
            TicketFacet::Type => &mut self.ticket_type,
            TicketFacet::Assignee => &mut self.assignee,
            TicketFacet::Priority => &mut self.priority,
        }
    }
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
    /// A custom field named by `sort_field` (CF5): number, date, date and
    /// time, choice and version (in the order of the field's choices; a
    /// version not among them after them, by its text) and checkbox (no,
    /// then yes). Empty values last.
    Field,
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

/// What a custom-field condition of a list asks (CF5, `cf`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "work/")]
pub enum CustomFieldOp {
    /// One of `values`: a choice, version or person (`me` for the reader);
    /// a multiple choice or several people match when they hold any.
    In,
    /// None of `values`; a ticket without a value matches.
    NotIn,
    /// Between `min` and `max`, both included, either open: a number
    /// (decimal), a date (`YYYY-MM-DD`) or a date and time (RFC 3339).
    Range,
    /// Holds no value (absent, `null`, `""` or `[]`).
    Empty,
    NotEmpty,
    /// A text, long text or URL that contains `value`, folded like the
    /// search (case and umlauts).
    Contains,
    /// A checkbox that is `value` (`true` or `false`; unset counts as
    /// `false`).
    Is,
}

/// One custom-field condition of a list: `cf` is a JSON array of these,
/// e.g. `[{"key":"device","op":"in","values":["ios"]}]`. Conditions add up
/// (all must hold). A key the workspace does not have, or an operator its
/// type does not take, is a `400`; a list with `cf` needs `workspace_id`
/// or `project_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct CustomFieldFilter {
    pub key: String,
    pub op: CustomFieldOp,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<String>>", optional)]
    pub values: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub min: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub max: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub value: Option<String>,
}

/// A ticket in a list (`fields=summary`): [`crate::work::TicketResponse`]
/// without `description`, `custom_values` and `triage_reason`. Open the
/// ticket (`GET /work/tickets/{id}`) for those, or name the custom fields
/// a list shows in `cf_keys` (CF5).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "work/")]
pub struct TicketSummary {
    pub id: String,
    pub project_id: String,
    pub sequence: i64,
    pub title: String,
    pub ticket_type: String,
    /// A status key (W4-F); see `TicketResponse.status`.
    pub status: String,
    #[serde(default)]
    #[ts(optional, as = "Option<crate::workflow::StatusCategory>")]
    pub status_category: crate::workflow::StatusCategory,
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
    /// The values of the custom fields `cf_keys` names (those the ticket
    /// holds); absent without `cf_keys`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "Record<string, unknown>")]
    pub custom_values: Option<serde_json::Map<String, serde_json::Value>>,
    /// Its attachments and its comments', not counting the trash (W4-B).
    #[serde(default)]
    #[ts(optional, as = "Option<u32>")]
    pub attachment_count: u32,
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
                facets: None,
            },
        };
        let json = serde_json::to_value(&page).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "data": [1], "meta": { "next_cursor": null, "limit": 50 } })
        );
    }
}
