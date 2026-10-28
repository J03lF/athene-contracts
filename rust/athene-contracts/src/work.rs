//! Work contracts: projects, tickets, comments, links, watchers, the inbox,
//! sprints, releases (milestones), labels, work logs and reports.
//!
//! These are the wire types of `/work/*`. The core reads requests and
//! builds responses with them, the gateway parses request bodies with them
//! before forwarding, and the TypeScript in `src/generated/work/` is
//! generated from them (see `tests/ts_bindings.rs`). The domain-to-wire
//! mappers stay in the core.
//!
//! Every response travels in the core's `{ "data": … }` envelope (the label
//! routes excepted); the types here are what sits inside it.
//!
//! Conventions:
//! - Request bodies reject unknown fields, so a field the contract does not
//!   know is a 400 at the edge instead of being dropped on the way.
//! - A PATCH field that can be cleared is a [`Patch`]: absent = unchanged,
//!   `null` = clear, a value = set.
//! - Ids are UUID strings, timestamps RFC 3339 strings, dates `YYYY-MM-DD`.
//! - Status, priority and relation fields in requests are strings the core
//!   parses (it accepts a few legacy spellings and any case); the
//!   TypeScript names the documented vocabulary.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::de::{Deserializer, Error as _};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use ts_rs::TS;

use crate::edit::{SetOps, TextHash};
use crate::Patch;

// ============================================================
// Vocabulary
// ============================================================

/// The built-in ticket statuses and the legacy `closed` (W4-F): the keys
/// tickets have always carried. A ticket's `status` is a **string** now —
/// the key of one of its workspace's statuses ([`crate::workflow`]), these
/// or one the workspace added — and what it means is its
/// `status_category`. `closed` is legacy: still read (as `done`), never
/// written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "work/")]
pub enum TicketStatus {
    Open,
    InProgress,
    InReview,
    Blocked,
    Done,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "work/")]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "work/")]
pub enum ProjectHealth {
    OnTrack,
    AtRisk,
    Blocked,
}

/// Whether a project takes work. An archived project (UX-020) is left out
/// of pickers, lists and counts, and it and everything in it is read-only:
/// a write answers `409` [`PROJECT_ARCHIVED_CODE`]. The one edit it takes
/// is `status: active` (restore); archiving completes its running sprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "work/")]
pub enum ProjectStatus {
    Active,
    Archived,
}

/// The code of the `409` a write into an archived project answers.
pub const PROJECT_ARCHIVED_CODE: &str = "ATH-RES-004";

/// The i18n key a client renders for [`PROJECT_ARCHIVED_CODE`].
pub const PROJECT_ARCHIVED_MESSAGE_KEY: &str = "errors.project_archived";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "work/")]
pub enum SprintStatus {
    Upcoming,
    Active,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "work/")]
pub enum MilestoneStatus {
    Open,
    Released,
}

/// How one ticket relates to another. Reads from the perspective of the
/// ticket that was asked about: `blocks` means *this* ticket blocks the
/// other one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "work/")]
pub enum LinkRelation {
    Blocks,
    BlockedBy,
    Related,
    Duplicates,
    DuplicatedBy,
    ChildOf,
    ParentOf,
}

/// Why an inbox entry reached the reader.
///
/// Ordered by how directly it concerns them: when several apply the
/// strongest wins, because "assigned to you" explains an entry better than
/// "you watch this" even when both are true.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "work/")]
pub enum InboxReason {
    Assigned,
    Reported,
    Watching,
    Mentioned,
    /// The ticket entered a queue whose target role the reader holds
    /// (custom roles C1, [`crate::queues`]).
    Queue,
}

// ============================================================
// Query strings
// ============================================================

/// `GET /work/tickets`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/", optional_fields)]
pub struct ListTicketsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// Only this workspace's tickets (403 for a non-member). Project and
    /// ticket keys are unique per workspace only, so a client showing one
    /// workspace should pass it, otherwise two tenants' `WR-1` can sit side
    /// by side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    // The list parameters below (step 3.9) make the answer a
    // `listing::Page`; see `crate::listing`. Lists are comma-separated.
    /// Status keys (`open,in_progress,qa,…`; `done` includes the legacy
    /// `closed`). A key no workspace has matches nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Status categories (`open,in_progress,blocked,done`; W4-F): tickets
    /// whose status means one of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// `me`, `none` (unassigned) or user ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    /// Priorities (`low,medium,high,critical`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    /// Ticket-type keys.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub ticket_type: Option<String>,
    /// Label ids: tickets carrying any of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Only the direct subtasks of this ticket (an epic's children).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// A sprint id, or `none` (in no sprint).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprint: Option<String>,
    /// Effective due date before this instant (RFC 3339, exclusive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_before: Option<String>,
    /// Effective due date at or after this instant (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_after: Option<String>,
    /// `true`: only tickets without an effective due date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_due: Option<bool>,
    /// Changed at or after this instant (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_since: Option<String>,
    /// Text in the key (`WEB-12`), title or description, as a substring;
    /// umlauts and case folded. At most 100 characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<crate::listing::TicketSort>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::listing::SortOrder>,
    /// `summary` (default) or `full`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<crate::listing::TicketFields>,
    /// Tickets per page, 1–200 (default 50).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// `meta.next_cursor` of the page before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// `true`: `meta.total` counts every match (one more statement).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<bool>,
    /// Custom-field conditions (CF5): a JSON array of
    /// [`crate::listing::CustomFieldFilter`], at most 10, at most 4000
    /// characters. Needs `workspace_id` or `project_id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cf: Option<String>,
    /// The custom field `sort=field` orders by (its key).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_field: Option<String>,
    /// Keys of the custom fields whose values a summary carries
    /// (`TicketSummary.custom_values`), at most 20, comma-separated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cf_keys: Option<String>,
}

impl ListTicketsQuery {
    /// Whether the query asks for a page: any parameter beyond the scope.
    /// Without one, the answer is the whole list (the old call).
    pub fn is_paged(&self) -> bool {
        self.status.is_some()
            || self.category.is_some()
            || self.assignee.is_some()
            || self.priority.is_some()
            || self.ticket_type.is_some()
            || self.label.is_some()
            || self.parent.is_some()
            || self.sprint.is_some()
            || self.due_before.is_some()
            || self.due_after.is_some()
            || self.no_due.is_some()
            || self.updated_since.is_some()
            || self.q.is_some()
            || self.sort.is_some()
            || self.order.is_some()
            || self.fields.is_some()
            || self.limit.is_some()
            || self.cursor.is_some()
            || self.count.is_some()
            || self.cf.is_some()
            || self.sort_field.is_some()
            || self.cf_keys.is_some()
    }
}

/// `GET /work/projects`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/", optional_fields)]
pub struct ListProjectsQuery {
    /// Only this workspace's projects (403 for a non-member).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
}

/// The per-user list `GET /work/activity` (the inbox: [`InboxQuery`]).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/", optional_fields)]
pub struct PerUserListQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// Only this workspace (403 for a non-member).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
}

/// `GET /work/inbox` (W5-A): the per-user list's parameters, and
/// `with_digests` for the caller's digests beside the entries.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/", optional_fields)]
pub struct InboxQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// Only this workspace (403 for a non-member).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    /// `true`: the answer carries `digests` (one more statement).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_digests: Option<bool>,
}

/// The per-user aggregates: `GET /work/stats` and `GET /work/my-tickets`.
/// Without `workspace_id`: every workspace the caller is a member of.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/", optional_fields)]
pub struct WorkScopeQuery {
    /// Only this workspace (403 for a non-member).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
}

/// `GET /work/projects/:id/reports`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/", optional_fields)]
pub struct ProjectReportQuery {
    /// Draw this sprint's burndown instead of the default one (the running
    /// sprint, else the last completed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprint_id: Option<String>,
}

// ============================================================
// Projects
// ============================================================

/// A project, as every `/work/projects` route returns it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "work/")]
pub struct ProjectResponse {
    pub id: String,
    /// The workspace the project belongs to. `null` for projects created
    /// before projects carried a workspace; those are invisible to
    /// workspace-scoped features (the activity feed, custom field
    /// definitions).
    pub workspace_id: Option<String>,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    /// Hex colour `#RRGGBB`. When `null` the frontend derives a stable
    /// colour from the key.
    pub color: Option<String>,
    pub health: ProjectHealth,
    /// 0..1: completion ratio, manually set or derived from tickets.
    pub progress: f32,
    pub lead_id: Option<String>,
    /// The team a new ticket of this project gets when the composer names
    /// none (custom fields CF0, E1). `null`: the creator's only team, if
    /// they are in exactly one.
    pub default_team_id: Option<String>,
    /// Who created the project. With the lead and the workspace's managers,
    /// the people who may edit its settings and overview. `null` for older
    /// projects whose creator the audit trail no longer records.
    pub created_by: Option<String>,
    pub status: ProjectStatus,
    /// Values for the workspace's `applies_to = project` field
    /// definitions, keyed by each definition's stable `key`. Always an
    /// object; `{}` when the workspace defines no project fields.
    #[ts(type = "Record<string, unknown>")]
    pub custom_values: JsonValue,
    /// Rises with every write to the project. Orders answers (a client
    /// drops one older than what it holds); it decides no conflict.
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// `POST /work/projects`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct CreateProjectRequest {
    pub key: String,
    pub name: String,
    /// Always send this. A project without a workspace records no activity
    /// and gets none of the workspace's custom fields; the field is optional
    /// only so older clients keep working.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Hex colour `#RRGGBB`; overrides the deterministic palette.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// `PATCH /work/projects/:id`.
///
/// Only the fields present are applied, so a patch that mentions nothing is
/// rejected rather than treated as "clear everything". `key` is not
/// editable: it prefixes every ticket ever raised in the project.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct UpdateProjectRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    /// `null` clears the description.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub description: Patch<String>,
    /// `#RRGGBB`, or `null` to fall back to the derived palette colour.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub color: Patch<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<ProjectHealth>")]
    pub health: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<ProjectStatus>")]
    pub status: Option<String>,
    /// `null` leaves the project without a lead.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub lead_id: Patch<String>,
    /// A team of the project's workspace; `null`: none.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub default_team_id: Patch<String>,
    /// Per changed field, the value the change is based on (step 3.7). A
    /// field whose stored value differs is a `409` `ATH-RES-003`; the other
    /// fields of the project are not touched either way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected: Option<ProjectExpected>,
}

/// The base of a project edit: the values the client saw, per field it
/// changes. Same keys as [`UpdateProjectRequest`]; `null` means "was empty".
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct ProjectExpected {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub description: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub color: Patch<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<ProjectHealth>")]
    pub health: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<ProjectStatus>")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub lead_id: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub default_team_id: Patch<String>,
}

/// `PATCH /work/projects/:id/custom-values` and
/// `PATCH /work/tickets/:id/custom-values`.
///
/// With `expected` the write is **partial** (custom fields CF1): only the
/// keys `expected` names are validated and written; a key absent from
/// `custom_values` or `null` there is emptied. A required field cannot be
/// emptied (for a ticket: required at create, or at close once closed),
/// every other field stays writable whatever else is missing, so an
/// existing ticket is never blocked. Without `expected` (stage A) the
/// object is the whole bucket, checked against the fields that apply.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct UpdateCustomValuesRequest {
    /// Missing or `null` is treated as `{}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = false, type = "Record<string, unknown>")]
    pub custom_values: Option<JsonValue>,
    /// Per key the client changes, the value it saw (`null`: the key was
    /// empty). With `expected`, only these keys are read and written: a key whose
    /// stored value moved is a `409` `ATH-RES-003` (`field` =
    /// `custom_values.<key>`), and every other key of `custom_values` is
    /// read as context, not as a change, so another person's edit of it
    /// survives. A multi-value key (an array) is merged as a set: what the
    /// client added to its base is added, what it removed is removed.
    /// Without `expected` (stage A) every key that differs is written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "Record<string, unknown>")]
    pub expected: Option<JsonValue>,
}

// ============================================================
// Tickets
// ============================================================

/// A ticket, as the ticket routes, the board and `my-tickets` return it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "work/")]
pub struct TicketResponse {
    pub id: String,
    pub project_id: String,
    /// Per-project monotonic counter. Combined with the project's `key` it
    /// produces the human-readable ticket key (e.g. `WEB-128`).
    pub sequence: i64,
    pub title: String,
    pub description: String,
    /// A ticket-type key of the workspace (`task`, `bug`, `feature`,
    /// `support`, `epic` or one the workspace added).
    pub ticket_type: String,
    /// The key of one of the workspace's statuses (W4-F): a built-in one
    /// (`open`, `in_progress`, `in_review`, `blocked`, `done`, the strings
    /// they always were), one the workspace added, or the legacy `closed`.
    pub status: String,
    /// What the status means: `open`, `in_progress`, `blocked` or `done`
    /// (legacy `closed` is `done`). Derived by the server in every read.
    #[serde(default)]
    #[ts(optional, as = "Option<crate::workflow::StatusCategory>")]
    pub status_category: crate::workflow::StatusCategory,
    pub priority: TicketPriority,
    pub assignee_id: Option<String>,
    pub reporter_id: String,
    pub sla_policy_id: Option<String>,
    /// Set when this ticket is a subtask. The subtask tree is built from
    /// this field, not from the link table, so a subtask has exactly one
    /// parent by construction and the server rejects cycles.
    pub parent_ticket_id: Option<String>,
    pub sprint_id: Option<String>,
    pub milestone_id: Option<String>,
    /// The team the ticket belongs to (custom fields CF0, E1); a team of
    /// the workspace, across projects. Its fields follow it.
    pub team_id: Option<String>,
    /// Fields hung onto this ticket alone ("+ Feld"), by definition id.
    pub adhoc_field_ids: Vec<String>,
    /// Planning estimate, 0..1000.
    pub story_points: Option<i32>,
    pub triage_category: Option<String>,
    pub triage_reason: Option<String>,
    pub triaged_at: Option<String>,
    pub first_response_at: Option<String>,
    pub resolved_at: Option<String>,
    pub due_date: Option<String>,
    /// When work on the ticket is planned to begin (W7-R, the roadmap):
    /// RFC 3339, like `due_date`. `null` when it has none; absent from an
    /// older core.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub start_date: Option<String>,
    pub sla_breached: bool,
    /// Custom field values keyed by `FieldDefinition.key`. Validated against
    /// the workspace's definitions before persistence; unknown keys are
    /// dropped. Always an object, `{}` when empty.
    #[ts(type = "Record<string, unknown>")]
    pub custom_values: JsonValue,
    /// Position within the board column, ordered inside (project, status).
    /// `null` for tickets written before ranking existed: sort a column by
    /// this value and place the nulls last, newest first.
    pub board_rank: Option<i64>,
    /// Who had the ticket when it last entered a queue (custom roles C1):
    /// "Zurückschicken" returns it to them. `null` when it never entered
    /// one, or entered unassigned.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub handoff_from: Option<String>,
    /// Who moved the ticket into its status (W4-G, the four-eyes rule
    /// `not_entered_by`): its reporter for a new ticket, `null` when nobody
    /// moved it since W4-G.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub status_entered_by: Option<String>,
    /// The deadline the ticket has: its own due date, else its running or
    /// planned sprint's end (`listing::EffectiveDue`). Computed by the
    /// server in every read; `null` without either.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub effective_due: Option<crate::listing::EffectiveDue>,
    /// Its attachments and its comments', not counting the trash (W4-B).
    #[serde(default)]
    #[ts(optional, as = "Option<u32>")]
    pub attachment_count: u32,
    /// Rises with every write to the ticket (also a background job's).
    /// Orders answers: a client drops one older than what it holds. It
    /// decides no conflict; `expected` does.
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// `POST /work/tickets`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct CreateTicketRequest {
    pub project_id: String,
    pub title: String,
    pub description: String,
    /// `low`, `medium`, `high` or `critical`.
    pub priority: String,
    /// A live ticket-type key of the project's workspace. Absent = the
    /// workspace's default type. Required custom fields and defaults follow
    /// it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ticket_type: Option<String>,
    /// Planning estimate. Required when the workspace's `require_estimate`
    /// setting is on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub story_points: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sla_policy_id: Option<String>,
    /// RFC 3339 timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    /// Makes the new ticket a subtask of this one, in the same request, so a
    /// subtask is never briefly a top-level ticket. The parent must exist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_ticket_id: Option<String>,
    /// Custom field values keyed by `FieldDefinition.key`; defaults to `{}`.
    /// Validated against the fields that apply to the new ticket (its team,
    /// type, project and `adhoc_field_ids`): 400 for type mismatches or
    /// missing fields required at create.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "Record<string, unknown>")]
    pub custom_values: Option<JsonValue>,
    /// A team of the workspace. Absent: the project's default team, else
    /// the creator's only team; `null`: no team.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub team_id: Patch<String>,
    /// Active ticket fields to hang onto the new ticket alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub adhoc_field_ids: Option<Vec<String>>,
}

/// Content patch for a ticket: `PATCH /work/tickets/:id`, and the `patch`
/// of a bulk edit.
///
/// Only the keys present are touched. For the nullable fields that matters
/// twice over: **omit** a key to leave it alone, send `null` to clear it.
/// `{ title }` alone does not wipe the due date; `{ due_date: null }` does.
///
/// Status and assignee are deliberately absent: they have their own
/// endpoints because they carry workflow side effects (SLA stamps,
/// notifications) that a content edit should not trigger. Custom values
/// have their own endpoint too, which validates them against the ticket's
/// type.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct UpdateTicketRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ticket_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<TicketPriority>")]
    pub priority: Option<String>,
    /// RFC 3339 timestamp, or `null` to clear.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub due_date: Patch<String>,
    /// The planned start (W7-R), RFC 3339, or `null` to clear. Not after
    /// the due date the ticket has with this patch applied (400).
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub start_date: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub story_points: Patch<i32>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub parent_ticket_id: Patch<String>,
    /// The release the ticket ships in; `null` takes it out of any.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub milestone_id: Patch<String>,
    /// A team of the ticket's workspace; `null` takes it out of any. Values
    /// stay; the fields follow the new team (custom fields CF0).
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub team_id: Patch<String>,
    /// Not editable here: the core answers 400 and names the custom-values
    /// endpoint. Declared so the refusal can say so; not part of the
    /// TypeScript.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(skip)]
    pub custom_values: Option<JsonValue>,
}

impl UpdateTicketRequest {
    /// `true` when the patch names no field.
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// The content patch of a ticket edit (`TicketEdit.patch`). The wire type
/// keeps its name, `UpdateTicketRequest`, so clients built against it still
/// compile.
pub type TicketPatch = UpdateTicketRequest;

/// The base of a ticket edit: per field the client changes, the value it
/// saw before the change (step 3.7, `CONFLICTS.md`).
///
/// One shape for every ticket edit endpoint (content patch, status, move,
/// assign, triage, bulk), keyed by the ticket's own field names as
/// `TicketResponse` has them. A key that is absent is not checked; `null`
/// means "was empty". The description is a long text and is expected as
/// the [`TextHash`] of the text the client started from.
///
/// The server checks only fields the request changes. When one of them no
/// longer holds its expected value, nothing is written and the answer is a
/// `409` `ATH-RES-003` naming that field.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct TicketExpected {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<TextHash>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ticket_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<TicketPriority>")]
    pub priority: Option<String>,
    /// A status key as the client saw it (legacy `closed` included).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub assignee_id: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub due_date: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub start_date: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub story_points: Patch<i32>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub parent_ticket_id: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub milestone_id: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub team_id: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub sla_policy_id: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub triage_category: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub triage_reason: Patch<String>,
    /// The project the client saw the ticket in: the base of a move to
    /// another project (`POST /work/tickets/:id/project`, bulk `project_id`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub project_id: Option<String>,
}

/// A ticket edit: `PATCH /work/tickets/:id` (step 3.7).
///
/// `patch` names the fields to change, `expected` their base. Labels and
/// watchers are sets and change by operations: two people adding labels at
/// once both keep theirs, so they need no base. `watchers` takes only the
/// caller's own id, as `PUT|DELETE …/watchers/me` do.
///
/// Stage A also accepts the old body, the bare [`UpdateTicketRequest`],
/// as an edit without `expected` ([`UpdateTicketBody`]).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct TicketEdit {
    /// May be left out for an edit of labels or watchers only.
    #[serde(default, skip_serializing_if = "UpdateTicketRequest::is_empty")]
    #[ts(optional, as = "Option<UpdateTicketRequest>")]
    pub patch: TicketPatch,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected: Option<TicketExpected>,
    /// Label ids of the ticket's workspace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub labels: Option<SetOps<String>>,
    /// User ids; only the caller's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub watchers: Option<SetOps<String>>,
}

/// The keys that mark a body as a [`TicketEdit`] rather than the stage-A
/// bare patch. None of them is a field of [`UpdateTicketRequest`].
const TICKET_EDIT_KEYS: [&str; 4] = ["patch", "expected", "labels", "watchers"];

/// The body of `PATCH /work/tickets/:id`: a [`TicketEdit`], or during rollout
/// stage A the bare [`UpdateTicketRequest`] older clients send (an edit
/// without `expected`). Both shapes reject unknown fields; which one a body
/// is follows from its keys. Not exported to TypeScript: clients send a
/// `TicketEdit`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum UpdateTicketBody {
    Edit(Box<TicketEdit>),
    Legacy(Box<UpdateTicketRequest>),
}

impl UpdateTicketBody {
    /// The edit either shape asks for.
    pub fn into_edit(self) -> TicketEdit {
        match self {
            Self::Edit(edit) => *edit,
            Self::Legacy(patch) => TicketEdit {
                patch: *patch,
                ..TicketEdit::default()
            },
        }
    }
}

impl<'de> Deserialize<'de> for UpdateTicketBody {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = serde_json::Map::<String, JsonValue>::deserialize(deserializer)?;
        let edit = TICKET_EDIT_KEYS.iter().any(|key| map.contains_key(*key));
        let value = JsonValue::Object(map);
        if edit {
            TicketEdit::deserialize(value).map(|edit| Self::Edit(Box::new(edit)))
        } else {
            UpdateTicketRequest::deserialize(value).map(|patch| Self::Legacy(Box::new(patch)))
        }
        .map_err(D::Error::custom)
    }
}

/// One patch applied to a selection of tickets: `POST /work/tickets/bulk`.
///
/// The same patch shape as a single edit, so a field the one accepts cannot
/// quietly be ignored by the other.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct BulkUpdateTicketsRequest {
    pub ticket_ids: Vec<String>,
    #[serde(default)]
    pub patch: UpdateTicketRequest,
    /// Taken apart from the patch and applied through the status endpoint's
    /// own path: a status change stamps SLA timers and clears a resolution
    /// date, and writing it as a plain field would skip all of that. A
    /// status key of each ticket's workspace (W4-F): a ticket whose
    /// workspace lacks it, or hides it in the ticket's project, lands in
    /// `failed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub status: Option<String>,
    /// The base per ticket, keyed by ticket id: each ticket is checked and
    /// written on its own, so a stale one lands in `conflicts` and the rest
    /// are written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected: Option<BTreeMap<String, TicketExpected>>,
    /// The assignee of every ticket; `null` unassigns. Through the assign
    /// endpoint's own path (a member, never a guest; its base is
    /// `expected.<id>.assignee_id`).
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub assignee_id: Patch<String>,
    /// Label operations on every ticket (ids of the tickets' workspace). A
    /// set: no base, no conflict.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub labels: Option<SetOps<String>>,
    /// Move every ticket to this project of the same workspace, as
    /// `POST /work/tickets/:id/project` does (base:
    /// `expected.<id>.project_id`). Nothing else may change with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub project_id: Option<String>,
    /// An owner's or admin's reason to move past a blocking WIP limit
    /// (W4-G, `status_rules`); every other caller is refused as without.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub wip_override: Option<crate::status_rules::WipOverride>,
}

/// One custom field set on a selection: `POST /work/tickets/bulk/fields`
/// (CF5, "Feld setzen").
///
/// One statement writes every ticket whose base holds, the way the bulk
/// edit answers: `updated`, `conflicts` (`fields: ["custom_values.<key>"]`)
/// and `failed` (the field does not apply to the ticket, a required field
/// would be emptied, …). The field must be one of the tickets' workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct BulkSetFieldRequest {
    pub ticket_ids: Vec<String>,
    /// The field's key.
    pub key: String,
    /// The new value as `custom_values` holds it; `null` empties the
    /// field. Absent with `ops`.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[ts(optional, type = "unknown")]
    pub value: Patch<JsonValue>,
    /// A multiple choice or several people: what to add and remove on
    /// every ticket, as a set (no base needed, no conflict).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ops: Option<SetOps<String>>,
    /// Per ticket id, the value the client saw (`null`: empty). A ticket
    /// whose value moved since is a conflict and keeps it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "Record<string, unknown>")]
    pub expected: Option<BTreeMap<String, JsonValue>>,
}

/// Why one ticket in a bulk edit did not move.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct BulkUpdateFailure {
    pub ticket_id: String,
    /// The error code the single edit would have answered
    /// (`ATH-RES-001`, `ATH-AUTH-002`, `ATH-VAL-001`, …).
    pub code: String,
    pub message: String,
    /// The `details.reason` the single edit would have answered (W4-K):
    /// an approval (`approval_not_allowed`, `approval_four_eyes`,
    /// `approval_target_not_allowed`), a blocking WIP limit
    /// (`wip_limit_reached`), a status the ticket's workspace or project
    /// does not take (`status_unknown`, `status_archived`,
    /// `status_hidden`), … Absent when the refusal names none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reason: Option<String>,
    /// The rest of that refusal's `details` (`status`, `to`, `limit`,
    /// `current`, `scope`, `place`, `assignee_id`, `fields`, …), without
    /// `reason`. Only a client error carries any; absent when empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "Record<string, unknown>")]
    pub details: Option<JsonValue>,
}

impl BulkUpdateFailure {
    /// The failure of `ticket_id` from a refusal's code, sentence and
    /// `details` (a 4xx's; a server error passes `None`): `reason` taken
    /// out of the details when it is a string, the rest kept when there is
    /// any.
    pub fn from_refusal(
        ticket_id: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
        details: Option<&JsonValue>,
    ) -> Self {
        let mut rest = details.and_then(JsonValue::as_object).cloned();
        let reason = rest.as_mut().and_then(|map| match map.remove("reason") {
            Some(JsonValue::String(reason)) => Some(reason),
            Some(other) => {
                map.insert("reason".to_string(), other);
                None
            }
            None => None,
        });
        Self {
            ticket_id: ticket_id.into(),
            code: code.into(),
            message: message.into(),
            reason,
            details: rest.filter(|map| !map.is_empty()).map(JsonValue::Object),
        }
    }
}

/// One ticket of a bulk edit whose base no longer held: nothing of it was
/// written. `fields` as in the `409` of a single edit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct BulkUpdateConflict {
    pub ticket_id: String,
    pub fields: Vec<String>,
}

/// What a bulk edit did, per ticket.
///
/// Deliberately not all-or-nothing: a selection of fifty where one has been
/// deleted under the user moves the other forty-nine and names the one that
/// did not. `updated` replaces what the client holds, `conflicts` are the
/// tickets someone else changed in the meantime (re-read and decide),
/// `failed` is what to tell the user about. Always HTTP 200.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "work/")]
pub struct BulkUpdateTicketsResponse {
    pub updated: Vec<TicketResponse>,
    pub conflicts: Vec<BulkUpdateConflict>,
    pub failed: Vec<BulkUpdateFailure>,
    /// For a move to another project: per moved ticket its keys and what
    /// was cleared (the moved tickets are in `updated` too).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<TicketProjectMove>>")]
    pub moved: Vec<TicketProjectMove>,
    /// What WIP limits warn about (W4-G), per ticket. A ticket a blocking
    /// limit, an approval rule or a missing right refused is in `failed`
    /// (`ATH-RES-002`, the reason in the message).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<crate::status_rules::TicketWarning>>")]
    pub warnings: Vec<crate::status_rules::TicketWarning>,
}

/// `POST /work/tickets/:id/project`: move a ticket to another project of
/// the same workspace (UX-008).
///
/// It needs `ticket.edit` on the ticket and `ticket.create` in the target.
/// The ticket gets the target's next number; its old key keeps finding it
/// (a key lookup, the search). Comments, history, watchers, links, labels,
/// the assignee, the parent and the subtasks stay. Sprint and release
/// belong to the old project and are cleared (`cleared` names them).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct MoveTicketProjectRequest {
    pub project_id: String,
    /// The base: `project_id` as the client saw it. Another move in the
    /// meantime is a `409` `ATH-RES-003` naming `project_id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected: Option<TicketExpected>,
}

/// What a move to another project did.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "work/")]
pub struct TicketProjectMove {
    pub ticket_id: String,
    /// `WEB-12`: the key before the move; it still finds the ticket.
    pub previous_key: String,
    /// `APP-3`: the key now.
    pub key: String,
    /// The fields the move cleared because they belong to the old project:
    /// `sprint_id`, `milestone_id`.
    pub cleared: Vec<String>,
}

/// The answer of `POST /work/tickets/:id/project`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "work/")]
pub struct MoveTicketProjectResponse {
    pub ticket: TicketResponse,
    pub moved: TicketProjectMove,
}

/// What a ticket delete took with it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct DeletedTicketResponse {
    pub id: String,
    /// Subtasks removed along with the ticket, at any depth.
    pub subtasks: usize,
}

/// `PATCH /work/tickets/:id/status`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct UpdateTicketStatusRequest {
    /// A status key of the ticket's workspace (W4-F): live (not archived)
    /// and not hidden in the ticket's project, else `400` with
    /// `details.reason` `status_unknown`, `status_archived` or
    /// `status_hidden`. Older spellings still work (`prog`, `review`;
    /// `closed` and `resolved` set `done`).
    pub status: String,
    /// The base of the change (step 3.7): for `status`, the status the
    /// client saw. See [`TicketExpected`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected: Option<TicketExpected>,
    /// An owner's or admin's reason to move past a blocking WIP limit
    /// (W4-G). The status rules are `status_rules`: an approval refused is
    /// `409` `approval_*`, a blocking limit `409` `wip_limit_reached`; the
    /// answer carries `warnings` when a limit warns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub wip_override: Option<crate::status_rules::WipOverride>,
}

/// A board drag: `PATCH /work/tickets/:id/move`.
///
/// Status and position travel together because that is what the gesture
/// means; splitting them would let a column change persist while the
/// position failed, leaving the card visibly in the wrong place.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct MoveTicketRequest {
    /// Target column. Same vocabulary as `UpdateTicketStatusRequest.status`.
    pub status: String,
    /// Zero-based position within the target column, counted after the
    /// ticket has been removed from wherever it was. Clamped by the core
    /// (a negative index is the top), so a stale view lands the card at an
    /// end instead of failing.
    pub index: i64,
    /// The base of the column change: `status` as the client saw it. The
    /// position needs none (the last drag wins).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected: Option<TicketExpected>,
    /// As in [`UpdateTicketStatusRequest`] (W4-G).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub wip_override: Option<crate::status_rules::WipOverride>,
}

/// `PATCH /work/tickets/:id/assign`. `null` (or a missing key) unassigns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct AssignTicketRequest {
    #[serde(default)]
    pub assignee_id: Option<String>,
    /// The base: `assignee_id` as the client saw it (`null`: unassigned).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected: Option<TicketExpected>,
    /// An owner's or admin's reason to assign past a blocking WIP limit
    /// per person on the ticket's status (W4-K): the new assignee counts
    /// one more there. A limit that warns answers `warnings`, one that
    /// blocks `409` `wip_limit_reached`, as a status write does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub wip_override: Option<crate::status_rules::WipOverride>,
}

/// Triage a ticket: `PATCH /work/tickets/:id/triage`. Allowed to any member
/// of its workspace except guests.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct TriageTicketRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<TicketPriority>")]
    pub priority: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sla_policy_id: Option<String>,
    /// The base: the ticket's `triage_category` and `triage_reason` (both
    /// always written by a triage) and, when sent, `priority` and
    /// `sla_policy_id`, as the client saw them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected: Option<TicketExpected>,
}

/// Ticket and project counts for the dashboard: `GET /work/stats`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct WorkStatsResponse {
    pub open_tickets: usize,
    pub in_progress_tickets: usize,
    pub total_projects: usize,
}

/// One entry of `GET /work/activity`: a recent change from the audit trail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "work/")]
pub struct WorkActivityEntryResponse {
    pub actor_initials: String,
    pub actor_name: Option<String>,
    /// The audit verb (`work.ticket.created`, …). No server prose beside
    /// it any more (BE-17, 3.10): the client names it in the reader's
    /// language.
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    #[ts(type = "Record<string, unknown> | null")]
    pub details: Option<JsonValue>,
    pub created_at: String,
}

// ============================================================
// Inbox
// ============================================================

/// One inbox entry: something that happened, on a ticket the reader is
/// involved in.
///
/// Shaped like an activity event with the ticket attached, so the same
/// renderer serves both surfaces. The reader's own actions are never
/// included.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "work/")]
pub struct InboxEntryResponse {
    /// The activity event's id.
    pub id: String,
    pub actor_id: Option<String>,
    /// Display name of the actor, resolved by the server. `null` for system
    /// events and deleted accounts.
    pub actor_name: Option<String>,
    /// An activity verb (`ActivityAction`).
    pub action: String,
    pub detail: Option<String>,
    #[ts(type = "Record<string, unknown> | null")]
    pub metadata: Option<JsonValue>,
    /// RFC 3339.
    pub created_at: String,
    pub reason: InboxReason,
    pub ticket: TicketResponse,
}

/// `GET /work/inbox` (W5-A): the entries, and with `with_digests=true`
/// the caller's digests of the same workspaces (each counts as one more
/// unread entry until it is read: the same `inbox_state` rules as an
/// entry, by its `id` and `created_at`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "work/")]
pub struct InboxResponse {
    pub data: Vec<InboxEntryResponse>,
    /// Absent unless asked for; newest first, only digests with tickets of
    /// the workspaces the inbox shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub digests: Option<Vec<crate::notifications::InboxDigest>>,
}

// ============================================================
// Comments
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct TicketCommentResponse {
    pub id: String,
    pub ticket_id: String,
    pub author_id: String,
    pub body: String,
    /// RFC 3339.
    pub created_at: String,
    /// RFC 3339; when the author last changed the text, `null` if never.
    pub edited_at: Option<String>,
}

/// `POST /work/tickets/:id/comments` and
/// `PATCH /work/tickets/:id/comments/:comment_id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct AddTicketCommentRequest {
    pub body: String,
}

// ============================================================
// Links and watchers
// ============================================================

/// One link on a ticket, with the ticket on the other end already
/// resolved.
///
/// The server writes the symmetric counterpart too, so the other ticket
/// shows the inverse without a second request. Parent/child hierarchy does
/// **not** come through here; it lives on `TicketResponse.parent_ticket_id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "work/")]
pub struct TicketLinkResponse {
    pub relation: LinkRelation,
    pub ticket: TicketResponse,
    pub created_at: String,
    pub created_by: Option<String>,
}

/// `POST /work/tickets/:id/links`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct CreateTicketLinkRequest {
    pub to_ticket_id: String,
    #[ts(as = "LinkRelation")]
    pub relation: String,
}

/// `DELETE /work/tickets/:id/links/:relation/:to_id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct TicketLinkDeletedResponse {
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct TicketWatcherResponse {
    pub user_id: String,
    /// `null` when the user record is gone; fall back to the id.
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub added_at: String,
}

/// The watcher list plus whether the caller is on it, so the list and the
/// watch toggle come from one request instead of two.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct TicketWatcherListResponse {
    pub watchers: Vec<TicketWatcherResponse>,
    pub watching: bool,
    /// How many watch the ticket (`watchers.len()`, W4-H).
    pub count: usize,
}

/// Response of the watch / unwatch toggle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct WatchStateResponse {
    pub watching: bool,
}

// ============================================================
// Sprints
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct SprintResponse {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub goal: Option<String>,
    /// `YYYY-MM-DD`
    pub start_date: String,
    /// `YYYY-MM-DD`
    pub end_date: String,
    pub goal_points: Option<i32>,
    pub status: SprintStatus,
    /// What the sprint held when it was completed (UX-005): a completed
    /// sprint's history reads this, not today's tickets. `null` while it
    /// runs or is planned, and for sprints completed before it was kept.
    pub completed_stats: Option<SprintCompletionStats>,
    /// Rises with every write to the sprint; orders answers, decides no
    /// conflict.
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// A completed sprint's numbers, frozen at completion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct SprintCompletionStats {
    pub total_tickets: i64,
    pub done_tickets: i64,
    pub total_points: i64,
    pub done_points: i64,
    /// Unfinished tickets that left the sprint at completion.
    pub carried_over: i64,
    /// The sprint they went to; `null`: the backlog.
    pub carried_to: Option<String>,
    /// RFC 3339.
    pub completed_at: String,
}

/// `POST /work/projects/:id/sprints`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct CreateSprintRequest {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
    /// `YYYY-MM-DD`
    pub start_date: NaiveDate,
    /// `YYYY-MM-DD`
    pub end_date: NaiveDate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal_points: Option<i32>,
}

/// Rename a sprint, change its goal or move its dates:
/// `POST /work/sprints/:id/update`.
///
/// Tickets without an own due date follow the end by themselves (their
/// deadline is the sprint's end, derived, never stored);
/// `move_due_ticket_ids` names tickets of this sprint whose own due date
/// moves by as many days as the end does.
///
/// The answer is the sprint in `data` and, beside it in the envelope,
/// `moved_tickets`: how many own due dates moved.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct UpdateSprintRequest {
    /// A new name (a planned or running sprint; UX-025).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    /// A new goal; `null` clears it.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub goal: Patch<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub start_date: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub end_date: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub move_due_ticket_ids: Vec<String>,
    /// The base of the dates that change (step 3.7). A date someone else
    /// moved in the meantime is a `409` `ATH-RES-003`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected: Option<SprintExpected>,
}

/// The dates a sprint edit is based on.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct SprintExpected {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub goal: Patch<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub start_date: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub end_date: Option<NaiveDate>,
}

/// Where a completed sprint's unfinished tickets go:
/// `POST /work/sprints/:id/complete`. Another sprint of the same project
/// that is not completed, or (absent or `null`) the backlog. The body
/// itself is optional.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct CompleteSprintRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_open_to: Option<String>,
}

/// `POST /work/sprints/:id/assign` and `/unassign`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct AssignTicketsRequest {
    pub ticket_ids: Vec<String>,
}

/// `GET /work/sprints/:id/burndown`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "work/")]
pub struct SprintBurnDownResponse {
    pub sprint_id: String,
    pub total_points: i32,
    pub done_points: i32,
    pub total_tickets: usize,
    pub done_tickets: usize,
    /// 0..100.
    pub completion_pct: f32,
}

// ============================================================
// Releases (milestones)
// ============================================================

/// How far a release is along.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct MilestoneProgress {
    pub tickets: usize,
    pub done: usize,
    pub points: i64,
    pub done_points: i64,
}

/// A release (Jira's version) with how far along it is.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct MilestoneResponse {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    /// `YYYY-MM-DD`, or `null` when no date is set.
    pub due_date: Option<String>,
    pub status: MilestoneStatus,
    pub created_at: String,
    pub updated_at: String,
    pub progress: MilestoneProgress,
}

/// `POST /work/projects/:id/milestones`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct CreateMilestoneRequest {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// `YYYY-MM-DD`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
}

/// `PATCH /work/milestones/:id`. An absent key leaves that part as it is;
/// `null` clears it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct UpdateMilestoneRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub description: Patch<String>,
    /// `YYYY-MM-DD`, or `null` to clear.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub due_date: Patch<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<MilestoneStatus>")]
    pub status: Option<String>,
}

// ============================================================
// Labels
// ============================================================

/// The label routes answer without the `{ "data": … }` envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct LabelResponse {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub color: Option<String>,
    pub created_at: String,
}

/// `POST /work/workspaces/:id/labels`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct CreateLabelRequest {
    pub name: String,
    /// `#RRGGBB`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Rename and/or recolour a label: `PATCH /work/labels/:id`. An absent key
/// leaves that part as it is; `color: null` clears the colour back to the
/// theme's default.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct UpdateLabelRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    /// `#RRGGBB`, or `null` to clear.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub color: Patch<String>,
}

/// One label on one ticket.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct LabelAssignmentResponse {
    pub ticket_id: String,
    pub label_id: String,
}

/// Replace a ticket's labels: `POST /work/tickets/:id/labels/set`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct SetLabelsRequest {
    pub label_ids: Vec<String>,
    /// Ignored: the workspace is the ticket's own. Still accepted, because
    /// clients sent it while the server trusted it for the activity scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
}

// ============================================================
// Work logs
// ============================================================

/// Time someone booked on a ticket.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct WorkLogResponse {
    pub id: String,
    pub ticket_id: String,
    pub user_id: String,
    pub minutes: i32,
    /// RFC 3339: when the stretch of work began.
    pub started_at: String,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct WorkLogListResponse {
    pub total_minutes: i64,
    /// Newest first.
    pub entries: Vec<WorkLogResponse>,
}

/// `POST /work/tickets/:id/worklogs`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct AddWorkLogRequest {
    pub minutes: i32,
    /// RFC 3339; defaults to the stretch that just ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

// ============================================================
// Reports
// ============================================================

/// One day of a sprint's burndown.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "work/")]
pub struct BurndownPointResponse {
    /// `YYYY-MM-DD`
    pub date: String,
    /// Points still open at the end of the day; `null` for days still ahead.
    pub remaining: Option<i64>,
    /// The straight line from the sprint's scope to zero.
    pub ideal: f64,
}

/// One completed sprint, for velocity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct VelocityEntryResponse {
    pub sprint_id: String,
    pub name: String,
    /// Points in the sprint when it closed; `null` if that was never
    /// recorded.
    pub committed: Option<i64>,
    pub completed: i64,
}

/// One day of the project's flow.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct FlowPointResponse {
    /// `YYYY-MM-DD`
    pub date: String,
    pub created: usize,
    pub resolved: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct StatusShareResponse {
    /// A status key (W4-F).
    pub status: String,
    #[serde(default)]
    #[ts(optional, as = "Option<crate::workflow::StatusCategory>")]
    pub category: crate::workflow::StatusCategory,
    pub count: usize,
    pub points: i64,
}

/// The sprint a report's burndown is drawn for.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/")]
pub struct ReportSprintResponse {
    pub id: String,
    pub name: String,
    /// `YYYY-MM-DD`
    pub start_date: String,
    /// `YYYY-MM-DD`
    pub end_date: String,
    pub status: SprintStatus,
}

/// Burndown, velocity and flow for a project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export_to = "work/")]
pub struct ProjectReportResponse {
    /// The sprint the burndown is drawn for, or `null` when there is none.
    pub sprint: Option<ReportSprintResponse>,
    pub burndown: Vec<BurndownPointResponse>,
    pub velocity: Vec<VelocityEntryResponse>,
    /// The last 28 days, oldest first.
    pub flow: Vec<FlowPointResponse>,
    pub statuses: Vec<StatusShareResponse>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn create_ticket_carries_story_points() {
        let body = json!({
            "project_id": "p", "title": "t", "description": "", "priority": "low",
            "story_points": 5
        });
        let req: CreateTicketRequest = serde_json::from_value(body.clone()).expect("create");
        assert_eq!(req.story_points, Some(5));
        // Forwarded as sent: no `null`s for the fields that were left out.
        assert_eq!(serde_json::to_value(&req).expect("serialise"), body);
    }

    #[test]
    fn requests_refuse_unknown_fields() {
        assert!(serde_json::from_value::<CreateTicketRequest>(json!({
            "project_id": "p", "title": "t", "description": "", "priority": "low",
            "storypoints": 5
        }))
        .is_err());
        assert!(
            serde_json::from_value::<UpdateTicketRequest>(json!({ "sprint_id": "s" })).is_err()
        );
        assert!(serde_json::from_value::<BulkUpdateTicketsRequest>(json!({
            "ticket_ids": [], "patch": { "titel": "x" }
        }))
        .is_err());
    }

    #[test]
    fn a_bulk_failure_names_the_reason_of_its_refusal() {
        let details = json!({
            "reason": "wip_limit_reached", "status": "qa", "limit": 2, "current": 2,
            "scope": "column", "place": "project", "assignee_id": null
        });
        let failure =
            BulkUpdateFailure::from_refusal("t1", "ATH-RES-002", "at the limit", Some(&details));
        assert_eq!(failure.reason.as_deref(), Some("wip_limit_reached"));
        let rest = failure.details.clone().expect("details");
        assert_eq!(rest["limit"], 2);
        assert!(rest.get("reason").is_none());
        let wire = serde_json::to_value(&failure).expect("serialise");
        assert_eq!(wire["reason"], "wip_limit_reached");
        assert_eq!(wire["details"]["status"], "qa");

        // A refusal without details (a missing ticket) says nothing more,
        // and an older answer without the two fields still reads.
        let plain = BulkUpdateFailure::from_refusal("t2", "ATH-RES-001", "not found", None);
        assert_eq!(
            serde_json::to_value(&plain).expect("serialise"),
            json!({ "ticket_id": "t2", "code": "ATH-RES-001", "message": "not found" })
        );
        let older: BulkUpdateFailure = serde_json::from_value(
            json!({ "ticket_id": "t3", "code": "ATH-RES-002", "message": "m" }),
        )
        .expect("older");
        assert_eq!((older.reason, older.details), (None, None));

        // Only `reason` alone: no empty `details`.
        let only = BulkUpdateFailure::from_refusal(
            "t4",
            "ATH-VAL-001",
            "hidden",
            Some(&json!({ "reason": "status_hidden" })),
        );
        assert_eq!(only.reason.as_deref(), Some("status_hidden"));
        assert_eq!(only.details, None);
    }

    #[test]
    fn an_assignment_may_carry_a_wip_override() {
        let req: AssignTicketRequest = serde_json::from_value(json!({
            "assignee_id": "u", "wip_override": { "reason": "Release tomorrow" }
        }))
        .expect("assign");
        assert_eq!(
            req.wip_override.map(|w| w.reason).as_deref(),
            Some("Release tomorrow")
        );
        assert!(serde_json::from_value::<AssignTicketRequest>(json!({
            "assignee_id": "u", "wip_override": { "reason": "x", "force": true }
        }))
        .is_err());
    }

    #[test]
    fn update_ticket_tells_absent_from_null() {
        let req: UpdateTicketRequest =
            serde_json::from_value(json!({ "title": "x", "due_date": null })).expect("patch");
        assert_eq!(req.due_date, Patch::Null);
        assert_eq!(req.story_points, Patch::Absent);
        assert_eq!(
            serde_json::to_value(&req).expect("serialise"),
            json!({ "title": "x", "due_date": null })
        );
    }

    #[test]
    fn a_ticket_edit_carries_patch_expected_and_set_ops() {
        let body = json!({
            "patch": { "title": "New", "due_date": null },
            "expected": { "title": "Old", "due_date": "2026-10-01T00:00:00Z" },
            "labels": { "add": ["l1"], "remove": ["l2"] }
        });
        let parsed: UpdateTicketBody = serde_json::from_value(body.clone()).expect("edit");
        let UpdateTicketBody::Edit(edit) = &parsed else {
            panic!("read as the legacy shape: {parsed:?}");
        };
        assert_eq!(edit.patch.title.as_deref(), Some("New"));
        assert_eq!(edit.patch.due_date, Patch::Null);
        let expected = edit.expected.as_ref().expect("expected");
        assert_eq!(expected.title.as_deref(), Some("Old"));
        assert_eq!(expected.story_points, Patch::Absent);
        assert_eq!(edit.labels.as_ref().map(|l| l.add.len()), Some(1));
        // Forwarded by the gateway exactly as sent.
        assert_eq!(serde_json::to_value(&parsed).expect("serialise"), body);
    }

    #[test]
    fn the_bare_patch_is_still_an_edit_without_expected() {
        let body = json!({ "title": "x", "story_points": 3 });
        let parsed: UpdateTicketBody = serde_json::from_value(body.clone()).expect("legacy");
        assert!(matches!(parsed, UpdateTicketBody::Legacy(_)));
        assert_eq!(serde_json::to_value(&parsed).expect("serialise"), body);
        let edit = parsed.into_edit();
        assert_eq!(edit.patch.story_points, Patch::Value(3));
        assert!(edit.expected.is_none());
    }

    #[test]
    fn both_edit_shapes_refuse_unknown_fields() {
        for body in [
            json!({ "titel": "x" }),
            json!({ "patch": { "titel": "x" } }),
            json!({ "patch": {}, "expected": { "titel": "x" } }),
            json!({ "patch": {}, "force": true }),
            json!({ "title": "x", "expected": { "title": "y" } }),
            json!({ "labels": { "put": ["a"] } }),
            json!({ "expected": { "description": "not-a-hash" } }),
        ] {
            assert!(
                serde_json::from_value::<UpdateTicketBody>(body.clone()).is_err(),
                "accepted {body}"
            );
        }
    }

    #[test]
    fn a_bulk_edit_carries_a_base_per_ticket() {
        let req: BulkUpdateTicketsRequest = serde_json::from_value(json!({
            "ticket_ids": ["a", "b"],
            "patch": { "priority": "high" },
            "expected": { "a": { "priority": "low" } }
        }))
        .expect("bulk");
        let expected = req.expected.expect("expected");
        assert_eq!(expected["a"].priority.as_deref(), Some("low"));
        assert!(!expected.contains_key("b"));
    }

    #[test]
    fn move_accepts_a_negative_index() {
        let req: MoveTicketRequest =
            serde_json::from_value(json!({ "status": "open", "index": -1 })).expect("move");
        assert_eq!(req.index, -1);
    }

    #[test]
    fn enums_use_the_wire_spelling() {
        assert_eq!(
            serde_json::to_value(TicketStatus::InProgress).expect("status"),
            json!("in_progress")
        );
        assert_eq!(
            serde_json::to_value(ProjectHealth::AtRisk).expect("health"),
            json!("at_risk")
        );
        assert_eq!(
            serde_json::to_value(LinkRelation::DuplicatedBy).expect("relation"),
            json!("duplicated_by")
        );
    }
}
