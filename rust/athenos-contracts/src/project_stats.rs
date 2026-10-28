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
//!
//! Each project also carries its health (W9-H, [`ProjectHealthState`]):
//! what its tickets say by the workspace's [`ProjectHealthRules`], with the
//! reasons, and the status every view shows — the one set by hand while
//! the project's mode is `manual`, else the computed one. It is judged in
//! the same statement as the numbers.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::work::{ProjectHealth, ProjectHealthMode};

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

/// A workspace's thresholds for a project's automatic health (W9-H),
/// `WorkspaceSettings::project_health` (`null` there = [`Self::default`]).
/// Each value lies in its bounds ([`PROJECT_HEALTH_BOUNDS`]).
///
/// - **Blocked**: an open ticket in a `blocked` status for longer than
///   `blocked_days` days, or any blocked ticket of the highest priority.
/// - **At risk**: overdue open tickets more than `overdue_percent` % of the
///   open ones, or at least `overdue_count`; the running sprint ends within
///   `horizon_days` days with more than `sprint_open_percent` % of its
///   tickets open; or an open release dated within `horizon_days` days
///   still has open tickets.
/// - Otherwise, and always without open tickets: **on track**.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "work/")]
pub struct ProjectHealthRules {
    /// Days in a blocked status before a ticket blocks the project.
    pub blocked_days: u32,
    /// Overdue share of the open tickets above which the project is at risk.
    pub overdue_percent: u32,
    /// Overdue tickets from which the project is at risk whatever the share.
    pub overdue_count: u32,
    /// Days before a sprint's end or a release's date that count as close.
    pub horizon_days: u32,
    /// Open share of the running sprint's tickets above which its close
    /// end puts the project at risk.
    pub sprint_open_percent: u32,
}

/// The defaults agreed for W9-H.
pub const PROJECT_HEALTH_DEFAULT_RULES: ProjectHealthRules = ProjectHealthRules {
    blocked_days: 2,
    overdue_percent: 10,
    overdue_count: 3,
    horizon_days: 2,
    sprint_open_percent: 30,
};

/// The smallest and largest value of each [`ProjectHealthRules`] field.
pub const PROJECT_HEALTH_BOUNDS: [(&str, u32, u32); 5] = [
    ("blocked_days", 1, 60),
    ("overdue_percent", 1, 100),
    ("overdue_count", 1, 100),
    ("horizon_days", 0, 30),
    ("sprint_open_percent", 1, 100),
];

impl Default for ProjectHealthRules {
    fn default() -> Self {
        PROJECT_HEALTH_DEFAULT_RULES
    }
}

impl ProjectHealthRules {
    fn fields(&self) -> [(&'static str, u32); 5] {
        [
            ("blocked_days", self.blocked_days),
            ("overdue_percent", self.overdue_percent),
            ("overdue_count", self.overdue_count),
            ("horizon_days", self.horizon_days),
            ("sprint_open_percent", self.sprint_open_percent),
        ]
    }

    /// The first field outside its bounds, as a message.
    pub fn check(&self) -> Result<(), String> {
        for ((name, value), (_, min, max)) in self.fields().into_iter().zip(PROJECT_HEALTH_BOUNDS) {
            if value < min || value > max {
                return Err(format!(
                    "project_health.{name} must be between {min} and {max}"
                ));
            }
        }
        Ok(())
    }
}

/// Why the tickets put a project where they do (W9-H). The web words it
/// ("3 überfällig", "1 blockiert seit 4 Tagen").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "work/")]
pub enum ProjectHealthReasonCode {
    /// `count` tickets blocked for longer than the threshold; `days` the
    /// longest of them, in whole days.
    BlockedStale,
    /// `count` blocked tickets of the highest priority.
    BlockedCritical,
    /// `count` overdue open tickets, `percent` of the open ones.
    Overdue,
    /// The running sprint (`label`) ends in `days` days (0 = today,
    /// negative = past its end) with `percent` % of its `count` tickets
    /// open.
    SprintEnding,
    /// The release `label` is due in `days` days (negative = past its date)
    /// with `count` open tickets.
    ReleaseDue,
}

/// One reason of a verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "work/")]
pub struct ProjectHealthReason {
    pub code: ProjectHealthReasonCode,
    /// The status this reason alone makes (`blocked` or `at_risk`).
    pub status: ProjectHealth,
    pub count: u64,
    pub days: Option<i64>,
    pub percent: Option<u32>,
    /// The sprint's name or the release's title.
    pub label: Option<String>,
}

/// A project's health (W9-H).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "work/")]
pub struct ProjectHealthState {
    /// What every view shows and sorts by: the status set by hand in
    /// `manual` mode, else `computed`.
    pub status: ProjectHealth,
    pub mode: ProjectHealthMode,
    /// What the tickets say, in either mode.
    pub computed: ProjectHealth,
    /// Why `computed` is what it is, most severe first; empty when on track.
    pub reasons: Vec<ProjectHealthReason>,
    /// In `manual` mode: who set it (`null` when unknown).
    pub set_by: Option<String>,
    /// RFC 3339, in `manual` mode: when it was set.
    pub set_at: Option<String>,
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
    /// Its health (W9-H).
    pub health: ProjectHealthState,
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

    #[test]
    fn the_default_rules_lie_in_their_bounds_and_a_stray_value_is_named() {
        assert_eq!(ProjectHealthRules::default().check(), Ok(()));
        let low = ProjectHealthRules {
            blocked_days: 0,
            ..ProjectHealthRules::default()
        };
        assert!(low.check().is_err_and(|e| e.contains("blocked_days")));
        let high = ProjectHealthRules {
            sprint_open_percent: 101,
            ..ProjectHealthRules::default()
        };
        assert!(high
            .check()
            .is_err_and(|e| e.contains("sprint_open_percent")));
        // Every field is required, and no other is taken.
        assert!(serde_json::from_value::<ProjectHealthRules>(
            serde_json::json!({ "blocked_days": 2 })
        )
        .is_err());
        let mut all = serde_json::to_value(ProjectHealthRules::default()).unwrap_or_default();
        all["extra"] = serde_json::json!(1);
        assert!(serde_json::from_value::<ProjectHealthRules>(all).is_err());
    }

    #[test]
    fn a_reason_goes_over_the_wire_in_snake_case() {
        let reason = ProjectHealthReason {
            code: ProjectHealthReasonCode::BlockedStale,
            status: ProjectHealth::Blocked,
            count: 1,
            days: Some(4),
            percent: None,
            label: None,
        };
        let json = serde_json::to_value(reason).unwrap_or_default();
        assert_eq!(json["code"], "blocked_stale");
        assert_eq!(json["status"], "blocked");
        assert_eq!(
            serde_json::to_value(ProjectHealthMode::Manual).unwrap_or_default(),
            "manual"
        );
    }
}
