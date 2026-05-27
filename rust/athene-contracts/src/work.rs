//! Work-management DTOs: Sprints, Labels, TicketLinks, Watchers.
//!
//! These are the wire shapes used by `athene-api` and downstream clients.
//! Domain entities live in the athene crate; these mirror their public fields.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

// ============================================================
// Sprints
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SprintStatus {
    Upcoming,
    Active,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct SprintResponse {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub goal: Option<String>,
    /// ISO date (`YYYY-MM-DD`)
    pub start_date: String,
    pub end_date: String,
    pub goal_points: Option<i32>,
    pub status: SprintStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
pub struct CreateSprintRequest {
    #[validate(length(min = 1, max = 80))]
    pub name: String,
    pub goal: Option<String>,
    pub start_date: String, // YYYY-MM-DD
    pub end_date: String,
    pub goal_points: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct AssignTicketsRequest {
    pub ticket_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct SprintBurnDownResponse {
    pub sprint_id: String,
    pub total_points: i32,
    pub done_points: i32,
    pub total_tickets: usize,
    pub done_tickets: usize,
    pub completion_pct: f32,
}

pub type SprintListResponse = crate::ListResponse<SprintResponse>;

// ============================================================
// Labels
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct LabelResponse {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub color: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
pub struct CreateLabelRequest {
    #[validate(length(min = 1, max = 32))]
    pub name: String,
    /// `#RRGGBB`
    pub color: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SetLabelsRequest {
    pub label_ids: Vec<String>,
    /// Required if the activity-event scope can't be derived from the ticket.
    pub workspace_id: Option<String>,
}

pub type LabelListResponse = crate::ListResponse<LabelResponse>;

// ============================================================
// Ticket Links
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LinkRelation {
    Blocks,
    BlockedBy,
    Related,
    Duplicates,
    DuplicatedBy,
    ChildOf,
    ParentOf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct TicketLinkResponse {
    pub from_ticket_id: String,
    pub to_ticket_id: String,
    pub relation: LinkRelation,
    pub created_at: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateTicketLinkRequest {
    pub to_ticket_id: String,
    pub relation: LinkRelation,
}

pub type TicketLinkListResponse = crate::ListResponse<TicketLinkResponse>;

// ============================================================
// Watchers
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct WatcherResponse {
    pub ticket_id: String,
    pub user_id: String,
    pub added_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct AddWatcherRequest {
    pub user_id: String,
}

pub type WatcherListResponse = crate::ListResponse<WatcherResponse>;
