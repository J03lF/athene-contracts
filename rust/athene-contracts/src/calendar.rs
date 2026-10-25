//! Calendar contracts (UX9): events ("Termine") and the calendar's sprint
//! and release bands. The TypeScript in `src/generated/calendar/` is
//! generated from them (see `tests/ts_bindings.rs`). Request bodies refuse
//! unknown fields.
//!
//! - `GET    /work/workspaces/:id/events?from=&to=`: the events of a window.
//! - `POST   /work/workspaces/:id/events`: create one ([`CreateEventRequest`]).
//! - `PATCH  /work/events/:id`: a field-level edit ([`EventEdit`], 409
//!   `ATH-RES-003` on a stale base, as for tickets).
//! - `DELETE /work/events/:id`: the creator or a manager.
//! - `GET    /work/workspaces/:id/calendar?from=&to=`: sprint bands with
//!   their open counts, release markers and the caller's watched tickets
//!   ([`CalendarResponse`]).
//!
//! Every response is wrapped as `{ "data": … }`. `from` and `to` are
//! calendar days (`YYYY-MM-DD`, both included) in the reader's zone; at
//! most [`CALENDAR_MAX_WINDOW_DAYS`] apart.
//!
//! An event is either **timed** (two instants, stored as `timestamptz`) or
//! **all-day** (two calendar dates, stored as dates): an all-day event is
//! the same day for every reader, whatever their zone — never "midnight
//! UTC", which is the day before west of Greenwich.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::edit::{SetOps, TextHash};
use crate::work::{MilestoneStatus, SprintStatus};
use crate::Patch;

/// The longest title, in characters.
pub const EVENT_MAX_TITLE_CHARS: usize = 200;
/// The longest description, in characters.
pub const EVENT_MAX_DESCRIPTION_CHARS: usize = 4000;
/// At most this many participants per event.
pub const EVENT_MAX_PARTICIPANTS: usize = 100;
/// The longest an event may run, in days (end minus start).
pub const EVENT_MAX_SPAN_DAYS: i64 = 31;
/// The widest window a calendar read may ask for, in days (`to - from`).
/// A month view with its leading and trailing weeks needs 41.
pub const CALENDAR_MAX_WINDOW_DAYS: i64 = 62;

/// When an event takes place.
///
/// Read through [`EventWhenWire`], which refuses unknown keys (C3); ts-rs
/// reads no `deny_unknown_fields` on an enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export_to = "calendar/")]
pub enum EventWhen {
    /// From one instant to a later one (RFC 3339; written back in UTC).
    Timed {
        #[schema(value_type = String)]
        starts_at: DateTime<Utc>,
        #[schema(value_type = String)]
        ends_at: DateTime<Utc>,
    },
    /// Whole days, `start_day` to `end_day`, both included (`YYYY-MM-DD`).
    AllDay {
        #[schema(value_type = String)]
        start_day: NaiveDate,
        #[schema(value_type = String)]
        end_day: NaiveDate,
    },
}

/// The wire shape of [`EventWhen`] as it is read: the same tags and
/// fields, unknown keys refused.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum EventWhenWire {
    Timed {
        starts_at: DateTime<Utc>,
        ends_at: DateTime<Utc>,
    },
    AllDay {
        start_day: NaiveDate,
        end_day: NaiveDate,
    },
}

impl<'de> Deserialize<'de> for EventWhen {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match EventWhenWire::deserialize(deserializer)? {
            EventWhenWire::Timed { starts_at, ends_at } => Self::Timed { starts_at, ends_at },
            EventWhenWire::AllDay { start_day, end_day } => Self::AllDay { start_day, end_day },
        })
    }
}

/// Why an [`EventWhen`] is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventWhenError {
    /// The end is not after the start (timed) or before it (all-day).
    EndBeforeStart,
    /// Longer than [`EVENT_MAX_SPAN_DAYS`].
    TooLong,
}

impl EventWhenError {
    /// The validation message the core answers with.
    pub const fn message(self) -> &'static str {
        match self {
            Self::EndBeforeStart => "an event ends after it starts",
            Self::TooLong => "an event lasts at most 31 days",
        }
    }
}

impl EventWhen {
    /// The rule every event obeys: a timed one ends after it starts, an
    /// all-day one ends on or after its first day; neither lasts longer
    /// than [`EVENT_MAX_SPAN_DAYS`].
    pub fn check(&self) -> Result<(), EventWhenError> {
        let span = match *self {
            Self::Timed { starts_at, ends_at } => {
                if ends_at <= starts_at {
                    return Err(EventWhenError::EndBeforeStart);
                }
                ends_at - starts_at
            }
            Self::AllDay { start_day, end_day } => {
                if end_day < start_day {
                    return Err(EventWhenError::EndBeforeStart);
                }
                end_day - start_day
            }
        };
        if span > chrono::TimeDelta::days(EVENT_MAX_SPAN_DAYS) {
            return Err(EventWhenError::TooLong);
        }
        Ok(())
    }

    pub const fn is_all_day(&self) -> bool {
        matches!(self, Self::AllDay { .. })
    }
}

/// An event of a workspace: a meeting, a review, a demo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "calendar/")]
pub struct EventResponse {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    /// `None` when there is none (never the empty string).
    pub description: Option<String>,
    pub when: EventWhen,
    /// Workspace members, in the order they were added.
    pub participant_ids: Vec<String>,
    /// The ticket the event is about, if any (same workspace).
    pub ticket_id: Option<String>,
    /// The sprint the event belongs to, if any (same workspace).
    pub sprint_id: Option<String>,
    pub created_by: String,
    pub updated_by: String,
    /// Rises with every write; orders answers, decides no conflict.
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// `POST /work/workspaces/:id/events`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "calendar/")]
pub struct CreateEventRequest {
    /// 1..200 characters, one line.
    pub title: String,
    /// ≤ 4000 characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    pub when: EventWhen,
    /// Workspace members (guests included); ≤ 100.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<String>>")]
    pub participant_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ticket_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub sprint_id: Option<String>,
}

/// The fields of an event edit. An absent key leaves that part as it is;
/// `null` clears it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "calendar/")]
pub struct UpdateEventRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub description: Patch<String>,
    /// A move or a resize: the whole of when, start and end together.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub when: Option<EventWhen>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub ticket_id: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub sprint_id: Patch<String>,
}

impl UpdateEventRequest {
    /// `true` when the patch names no field.
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// The base of an event edit: per field the client changes, what it saw.
/// The description is a long text, expected as the [`TextHash`] of the
/// text the client started from (the hash of `""` when there was none);
/// `when` as the whole value; `null` for a link means "was not set".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "calendar/")]
pub struct EventExpected {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    #[ts(optional)]
    pub description: Option<TextHash>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub when: Option<EventWhen>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub ticket_id: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub sprint_id: Patch<String>,
}

/// `PATCH /work/events/:id`: the changed fields, their base and the
/// participants as set operations (two people adding different
/// participants at once both keep theirs, so they need no base). A field
/// someone else changed in the meantime is a `409` `ATH-RES-003` whose
/// `details.current` is the event as it is now.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "calendar/")]
pub struct EventEdit {
    #[serde(default, skip_serializing_if = "UpdateEventRequest::is_empty")]
    #[ts(optional, as = "Option<UpdateEventRequest>")]
    pub patch: UpdateEventRequest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected: Option<EventExpected>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    #[ts(optional)]
    pub participants: Option<SetOps<String>>,
}

/// Query of `GET /work/workspaces/:id/events` and `…/calendar`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "calendar/")]
pub struct CalendarWindowQuery {
    /// First day, `YYYY-MM-DD`.
    #[schema(value_type = String)]
    pub from: NaiveDate,
    /// Last day, included.
    #[schema(value_type = String)]
    pub to: NaiveDate,
}

/// The project's next planned sprint, the target of "in nächsten Sprint".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "calendar/")]
pub struct CalendarNextSprint {
    pub id: String,
    pub name: String,
}

/// A sprint across its days, with the open work that is due when it ends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "calendar/")]
pub struct CalendarSprintBand {
    pub sprint_id: String,
    pub project_id: String,
    pub name: String,
    /// `YYYY-MM-DD`
    pub start_date: String,
    /// `YYYY-MM-DD`
    pub end_date: String,
    #[schema(value_type = String)]
    pub status: SprintStatus,
    /// Open tickets of the sprint without a due date of their own: they
    /// inherit the sprint's end and are counted here, not drawn one by one.
    pub open_inherited: u32,
    /// Of those, the ones assigned to the caller ("bei dir").
    pub open_inherited_mine: u32,
    /// The project's next upcoming sprint, if one is planned.
    pub next_sprint: Option<CalendarNextSprint>,
}

/// A release on its date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "calendar/")]
pub struct CalendarRelease {
    pub milestone_id: String,
    pub project_id: String,
    pub title: String,
    /// `YYYY-MM-DD`
    pub due_date: String,
    #[schema(value_type = String)]
    pub status: MilestoneStatus,
}

/// `GET /work/workspaces/:id/calendar`: what the calendar draws besides
/// tickets and events, read in one statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "calendar/")]
pub struct CalendarResponse {
    /// The window asked for, `YYYY-MM-DD`.
    pub from: String,
    pub to: String,
    /// Sprints overlapping the window, by start.
    pub sprints: Vec<CalendarSprintBand>,
    /// Releases due in the window, by date.
    pub releases: Vec<CalendarRelease>,
    /// Open tickets of the workspace the caller watches that have a due
    /// date of their own: the "watched by me" half of the calendar's
    /// "mine" filter (the ticket list does not carry watchers).
    pub watched_ticket_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, d).unwrap_or_default()
    }

    #[test]
    fn when_is_tagged_and_all_day_travels_as_dates() {
        let when = EventWhen::AllDay {
            start_day: day(26),
            end_day: day(27),
        };
        let json = serde_json::to_value(when).unwrap_or_default();
        assert_eq!(
            json,
            serde_json::json!({ "kind": "all_day", "start_day": "2026-10-26", "end_day": "2026-10-27" })
        );
        let timed: EventWhen = serde_json::from_value(serde_json::json!({
            "kind": "timed",
            "starts_at": "2026-10-26T10:00:00+01:00",
            "ends_at": "2026-10-26T10:30:00+01:00"
        }))
        .unwrap_or(when);
        assert_eq!(
            timed,
            EventWhen::Timed {
                starts_at: Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap(),
                ends_at: Utc.with_ymd_and_hms(2026, 10, 26, 9, 30, 0).unwrap(),
            }
        );
        for refused in [
            serde_json::json!({ "kind": "all_day", "start_day": "2026-10-26" }),
            serde_json::json!({ "kind": "all_day", "start_day": "2026-10-26T00:00:00Z", "end_day": "2026-10-26" }),
            serde_json::json!({ "kind": "weekly", "start_day": "2026-10-26", "end_day": "2026-10-26" }),
            serde_json::json!({ "kind": "all_day", "start_day": "2026-10-26", "end_day": "2026-10-26", "tz": "x" }),
        ] {
            assert!(
                serde_json::from_value::<EventWhen>(refused.clone()).is_err(),
                "{refused}"
            );
        }
    }

    #[test]
    fn when_checks_order_and_span() {
        let at = |h: u32| Utc.with_ymd_and_hms(2026, 10, 26, h, 0, 0).unwrap();
        assert_eq!(
            EventWhen::Timed {
                starts_at: at(10),
                ends_at: at(10)
            }
            .check(),
            Err(EventWhenError::EndBeforeStart)
        );
        assert!(EventWhen::Timed {
            starts_at: at(10),
            ends_at: at(11)
        }
        .check()
        .is_ok());
        assert!(EventWhen::AllDay {
            start_day: day(26),
            end_day: day(26)
        }
        .check()
        .is_ok());
        assert_eq!(
            EventWhen::AllDay {
                start_day: day(26),
                end_day: day(25)
            }
            .check(),
            Err(EventWhenError::EndBeforeStart)
        );
        let far = NaiveDate::from_ymd_opt(2026, 11, 27).unwrap_or_default();
        assert_eq!(
            EventWhen::AllDay {
                start_day: day(26),
                end_day: far
            }
            .check(),
            Err(EventWhenError::TooLong)
        );
    }

    #[test]
    fn an_edit_refuses_unknown_fields_and_reads_set_operations() {
        let edit: EventEdit = serde_json::from_value(serde_json::json!({
            "patch": { "title": "Review", "ticket_id": null },
            "expected": { "title": "Rev" },
            "participants": { "add": ["a"] }
        }))
        .unwrap_or_default();
        assert_eq!(edit.patch.title.as_deref(), Some("Review"));
        assert_eq!(edit.patch.ticket_id, Patch::Null);
        assert_eq!(
            edit.participants.map(|p| p.add),
            Some(vec!["a".to_string()])
        );
        for refused in [
            serde_json::json!({ "patch": { "titel": "x" } }),
            serde_json::json!({ "title": "x" }),
            serde_json::json!({ "expected": { "description": "short" } }),
        ] {
            assert!(
                serde_json::from_value::<EventEdit>(refused.clone()).is_err(),
                "{refused}"
            );
        }
    }
}
