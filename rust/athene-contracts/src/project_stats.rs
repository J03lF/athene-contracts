//! What a project looks like at a glance (B2, W7-P): per project the
//! tickets per status category, the points, the people with open work,
//! the dates that press and the last change — counted by the server in one
//! statement, so an overview never reads the tickets themselves. The
//! TypeScript in `src/generated/work/` is generated from these types (see
//! `tests/ts_bindings.rs`).
//!
//! - `GET /work/projects/stats?workspace_id=`: every project of the
//!   workspace that the projects list shows (archived ones too), each with
//!   its numbers, `{ "data": ProjectStatsResponse }`. Without
//!   `workspace_id`: every workspace the caller belongs to. `403` for a
//!   workspace the caller is no member of.
//! - `GET /work/projects/stats?project_id=`: that one project (the
//!   project's read rule; `404`/`403` as reading it).
//!
//! A project without tickets is in the answer with zeros. Every number is
//! of the project's own tickets (subtasks are tickets too), whatever their
//! sprint. "Open" is every category but `done`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

/// The most people one project's stats name (largest open load first).
pub const PROJECT_STATS_MAX_PEOPLE: usize = 8;

/// Days ahead that count as "due soon".
pub const PROJECT_STATS_DUE_SOON_DAYS: i64 = 7;

/// Days back that count as "recent" (`changed_recently`, `done_recently`).
pub const PROJECT_STATS_RECENT_DAYS: i64 = 7;

/// `GET /work/projects/stats`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/", optional_fields)]
pub struct ProjectStatsQuery {
    /// Only this workspace's projects (403 for a non-member).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    /// Only this project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

/// Tickets per status category (W4-F).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "work/")]
pub struct CategoryCounts {
    pub open: u64,
    pub in_progress: u64,
    pub blocked: u64,
    pub done: u64,
}

impl CategoryCounts {
    pub fn total(&self) -> u64 {
        self.open + self.in_progress + self.blocked + self.done
    }
}

/// Story points of a project: all of them, and those of finished tickets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "work/")]
pub struct PointCounts {
    pub total: i64,
    pub done: i64,
    /// Open tickets without an estimate.
    pub unestimated_open: u64,
}

/// One person with open work in the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "work/")]
pub struct ProjectPerson {
    pub user_id: String,
    /// Their open tickets in the project.
    pub open: u64,
}

/// The dates of a project's open tickets, by their effective due date.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "work/")]
pub struct DueCounts {
    /// Open, effective due date already past.
    pub overdue: u64,
    /// Open, due within the next [`PROJECT_STATS_DUE_SOON_DAYS`] days.
    pub due_soon: u64,
    /// RFC 3339: the earliest effective due date still ahead of an open
    /// ticket; `null` when none.
    pub next_due_at: Option<String>,
}

/// One project at a glance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "work/")]
pub struct ProjectStats {
    pub project_id: String,
    /// Every ticket of the project.
    pub total: u64,
    pub categories: CategoryCounts,
    pub points: PointCounts,
    pub due: DueCounts,
    /// The people with open tickets, most open first (then by id), at most
    /// [`PROJECT_STATS_MAX_PEOPLE`].
    pub people: Vec<ProjectPerson>,
    /// How many people have open tickets in all.
    pub people_count: u64,
    /// Open tickets nobody has.
    pub unassigned_open: u64,
    /// Tickets changed in the last [`PROJECT_STATS_RECENT_DAYS`] days.
    pub changed_recently: u64,
    /// Tickets finished in the last [`PROJECT_STATS_RECENT_DAYS`] days
    /// (by `resolved_at`).
    pub done_recently: u64,
    /// RFC 3339: the last change to any of its tickets; `null` without
    /// tickets.
    pub last_activity_at: Option<String>,
}

/// `GET /work/projects/stats`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "work/")]
pub struct ProjectStatsResponse {
    /// In the order of the projects list (by name).
    pub projects: Vec<ProjectStats>,
    /// RFC 3339: the server's clock the dates were judged by.
    pub as_of: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_are_snake_case_and_add_up() {
        let counts = CategoryCounts {
            open: 1,
            in_progress: 2,
            blocked: 3,
            done: 4,
        };
        assert_eq!(counts.total(), 10);
        let json = serde_json::to_value(counts).unwrap_or_default();
        assert_eq!(json["in_progress"], 2);
    }
}
