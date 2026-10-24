//! Dashboard contracts: the home page's overview, tiles and focus lists
//! (`/dashboard/*`).
//!
//! These answer without the `{ "data": … }` envelope. The TypeScript in
//! `src/generated/dashboard/` is generated from them (see
//! `tests/ts_bindings.rs`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::activity::ActivityEventResponse;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "dashboard/")]
pub struct KpiTile {
    pub label: String,
    pub value: i64,
    /// Change against the previous period, when there is one to compare.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub delta: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "dashboard/")]
pub struct KpisResponse {
    pub assigned: KpiTile,
    pub due_this_week: KpiTile,
    pub in_review: KpiTile,
    pub closed_this_week: KpiTile,
}

/// A ticket in a focus list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "dashboard/")]
pub struct FocusTicket {
    pub id: String,
    /// The ticket's per-project number. The key ("ATH-3") is this plus the
    /// project's prefix, composed on the client, which holds every project
    /// and one definition of what a ticket key looks like.
    pub sequence: i64,
    pub title: String,
    /// A `TicketStatus` value.
    pub status: String,
    /// A `TicketPriority` value.
    pub priority: String,
    pub assignee_id: Option<String>,
    pub project_id: String,
    /// RFC 3339.
    pub due_date: Option<String>,
}

/// `GET /dashboard/overview?workspace_id=`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema, TS)]
#[ts(export_to = "dashboard/")]
pub struct DashboardOverviewResponse {
    pub kpis: KpisResponse,
    /// The Today tab.
    pub focus: Vec<FocusTicket>,
    pub recent_activity: Vec<ActivityEventResponse>,
    pub on_track: OnTrackResponse,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "dashboard/")]
pub enum FocusTab {
    Today,
    Assigned,
    Review,
}

/// How the week is going.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "dashboard/")]
pub enum OnTrackState {
    /// At least one overdue item in the focus list; `count` is how many.
    Overdue,
    /// A heavy week; `count` items are due.
    BusyWeek,
    /// Nothing pressing; `count` is 0.
    OnTrack,
}

/// How the week is going, as a state plus a number.
///
/// Not a sentence: a sentence composed here could only ever be in one
/// language, and the client knows the reader's.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "dashboard/")]
pub struct OnTrackResponse {
    pub state: OnTrackState,
    /// Overdue items, or items due this week; `0` when on track.
    pub count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_focus_ticket_carries_its_sequence() {
        let ticket: FocusTicket = serde_json::from_value(json!({
            "id": "t", "sequence": 3, "title": "x", "status": "open", "priority": "low",
            "assignee_id": null, "project_id": "p", "due_date": null
        }))
        .expect("focus ticket");
        assert_eq!(ticket.sequence, 3);
    }

    #[test]
    fn the_week_state_uses_the_wire_spelling() {
        assert_eq!(
            serde_json::to_value(OnTrackState::BusyWeek).expect("state"),
            json!("busy_week")
        );
    }
}
