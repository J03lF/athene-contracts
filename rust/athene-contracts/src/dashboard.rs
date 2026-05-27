//! Dashboard aggregator DTOs.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::activity::ActivityEventResponse;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct KpiTile {
    pub label: String,
    pub value: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct KpisResponse {
    pub assigned: KpiTile,
    pub due_this_week: KpiTile,
    pub in_review: KpiTile,
    pub closed_this_week: KpiTile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct FocusTicket {
    pub id: String,
    pub key: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub assignee_id: Option<String>,
    pub project_id: String,
    pub due_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct DashboardOverviewResponse {
    pub kpis: KpisResponse,
    pub focus: Vec<FocusTicket>,
    pub recent_activity: Vec<ActivityEventResponse>,
    pub on_track_text: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FocusTab {
    Today,
    Assigned,
    Review,
}
