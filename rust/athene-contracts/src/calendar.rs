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
//! - `PATCH  /work/events/:id/occurrences`: an edit of one occurrence of a
//!   series with its scope ([`OccurrenceEdit`], W6-A).
//! - `POST   /work/events/:id/occurrences/delete`: delete one occurrence,
//!   it and the ones after it, or the series ([`OccurrenceDelete`]).
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
//!
//! A **series** (W6-A) is an event with a [`RecurrenceRule`] and an IANA
//! zone: its `when` is the first occurrence, the window read expands the
//! rest on the server, at the same wall-clock time in that zone.

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

// ------------------------------------------------------------
// Recurring events ("Serien", W6-A)
// ------------------------------------------------------------

/// Every N days/weeks/months/years: at most this N.
pub const RECURRENCE_MAX_INTERVAL: u16 = 99;
/// A series that ends "after N times" has at most this N.
pub const RECURRENCE_MAX_COUNT: u16 = 999;
/// At most this many occurrences of one series in one window read. A
/// daily series fills the widest window (63 days) with 63; the cap guards
/// the server, it never cuts a valid rule.
pub const SERIES_WINDOW_MAX_OCCURRENCES: usize = 100;
/// At most this many events and occurrences in one window read; the
/// earliest are kept.
pub const CALENDAR_MAX_ITEMS: usize = 2000;
/// The zone a series is anchored in when its creator has set none.
pub const SERIES_DEFAULT_TIME_ZONE: &str = "Europe/Zurich";

/// A day of the week, spelled as RFC 5545 spells it (`BYDAY`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "calendar/")]
pub enum Weekday {
    Mo,
    Tu,
    We,
    Th,
    Fr,
    Sa,
    Su,
}

impl Weekday {
    /// Monday first, as the week of a series starts (RFC 5545 `WKST=MO`).
    pub const ALL: [Self; 7] = [
        Self::Mo,
        Self::Tu,
        Self::We,
        Self::Th,
        Self::Fr,
        Self::Sa,
        Self::Su,
    ];

    /// Days after Monday, 0..=6.
    pub const fn index(self) -> u8 {
        match self {
            Self::Mo => 0,
            Self::Tu => 1,
            Self::We => 2,
            Self::Th => 3,
            Self::Fr => 4,
            Self::Sa => 5,
            Self::Su => 6,
        }
    }

    /// The day `index` days after Monday (`None` past Sunday).
    pub fn from_index(index: u8) -> Option<Self> {
        Self::ALL.get(usize::from(index)).copied()
    }
}

/// How often a series repeats (RFC 5545 `FREQ`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "calendar/")]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl RecurrenceFrequency {
    /// The text the database stores (`athene_events.rec_frequency`).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Yearly => "yearly",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "daily" => Some(Self::Daily),
            "weekly" => Some(Self::Weekly),
            "monthly" => Some(Self::Monthly),
            "yearly" => Some(Self::Yearly),
            _ => None,
        }
    }
}

/// Which day of the month a monthly series takes.
///
/// Read through `MonthlyRuleWire` (unknown keys refused, C3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, ToSchema, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export_to = "calendar/")]
pub enum MonthlyRule {
    /// The same day number every month: 1..=31, or -1 for the last day. A
    /// month without that day is skipped (RFC 5545: "monthly on the 31st"
    /// has no February); the last day never is.
    DayOfMonth { day: i8 },
    /// The nth weekday of the month: 1..=5, or -1 for the last ("2nd
    /// Tuesday", "last Friday"). A month without a 5th one is skipped.
    NthWeekday { nth: i8, weekday: Weekday },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum MonthlyRuleWire {
    DayOfMonth { day: i8 },
    NthWeekday { nth: i8, weekday: Weekday },
}

impl<'de> Deserialize<'de> for MonthlyRule {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match MonthlyRuleWire::deserialize(deserializer)? {
            MonthlyRuleWire::DayOfMonth { day } => Self::DayOfMonth { day },
            MonthlyRuleWire::NthWeekday { nth, weekday } => Self::NthWeekday { nth, weekday },
        })
    }
}

/// When a series stops.
///
/// Read through `RecurrenceEndWire` (unknown keys refused, C3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, ToSchema, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export_to = "calendar/")]
pub enum RecurrenceEnd {
    /// It goes on.
    Never,
    /// Its last occurrence starts on or before this day (`YYYY-MM-DD`, in
    /// the series' zone; RFC 5545 `UNTIL`).
    Until {
        #[schema(value_type = String)]
        day: NaiveDate,
    },
    /// After this many occurrences, the first included (RFC 5545 `COUNT`).
    Count { count: u16 },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum RecurrenceEndWire {
    Never,
    Until { day: NaiveDate },
    Count { count: u16 },
}

impl<'de> Deserialize<'de> for RecurrenceEnd {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match RecurrenceEndWire::deserialize(deserializer)? {
            RecurrenceEndWire::Never => Self::Never,
            RecurrenceEndWire::Until { day } => Self::Until { day },
            RecurrenceEndWire::Count { count } => Self::Count { count },
        })
    }
}

/// How a series repeats: a typed subset of the RFC 5545 `RRULE`
/// (`FREQ`, `INTERVAL`, `BYDAY`, `BYMONTHDAY`, `BYDAY` with an ordinal,
/// `UNTIL`, `COUNT`).
///
/// - `daily`: every `interval` days.
/// - `weekly`: every `interval` weeks on `weekdays` (Monday starts the
///   week). "Every weekday" is weekly on Monday to Friday. Without
///   `weekdays`: the weekday of the first occurrence.
/// - `monthly`: every `interval` months by `monthly`; without it, the day
///   of the month of the first occurrence.
/// - `yearly`: every `interval` years on the month and day of the first
///   occurrence (a series from 29 February is in leap years only).
///
/// The first occurrence is the event's `when`: a rule that does not take
/// that day starts on the next day it does (the server moves `when` there).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "calendar/")]
pub struct RecurrenceRule {
    pub frequency: RecurrenceFrequency,
    /// 1..=99: every N days, weeks, months or years.
    pub interval: u16,
    /// Weekly only: the days, in any order (answered Monday first).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<Weekday>>")]
    pub weekdays: Vec<Weekday>,
    /// Monthly only: by the day of the month or by the nth weekday.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub monthly: Option<MonthlyRule>,
    pub end: RecurrenceEnd,
}

/// Why a [`RecurrenceRule`] is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecurrenceError {
    Interval,
    WeekdaysOutsideWeekly,
    MonthlyOutsideMonthly,
    DayOfMonth,
    NthWeekday,
    Count,
}

impl RecurrenceError {
    /// The validation message the core answers with.
    pub const fn message(self) -> &'static str {
        match self {
            Self::Interval => "a series repeats every 1 to 99 days, weeks, months or years",
            Self::WeekdaysOutsideWeekly => "weekdays belong to a weekly series",
            Self::MonthlyOutsideMonthly => "a day of the month belongs to a monthly series",
            Self::DayOfMonth => "a day of the month is 1 to 31, or -1 for the last",
            Self::NthWeekday => "the nth weekday of a month is 1 to 5, or -1 for the last",
            Self::Count => "a series ends after 1 to 999 times",
        }
    }
}

impl RecurrenceRule {
    /// The shape every rule obeys, whatever its first occurrence.
    pub fn check(&self) -> Result<(), RecurrenceError> {
        if self.interval == 0 || self.interval > RECURRENCE_MAX_INTERVAL {
            return Err(RecurrenceError::Interval);
        }
        if !self.weekdays.is_empty() && self.frequency != RecurrenceFrequency::Weekly {
            return Err(RecurrenceError::WeekdaysOutsideWeekly);
        }
        match self.monthly {
            Some(_) if self.frequency != RecurrenceFrequency::Monthly => {
                return Err(RecurrenceError::MonthlyOutsideMonthly)
            }
            Some(MonthlyRule::DayOfMonth { day }) if day != -1 && !(1..=31).contains(&day) => {
                return Err(RecurrenceError::DayOfMonth)
            }
            Some(MonthlyRule::NthWeekday { nth, .. }) if nth != -1 && !(1..=5).contains(&nth) => {
                return Err(RecurrenceError::NthWeekday)
            }
            _ => {}
        }
        if let RecurrenceEnd::Count { count } = self.end {
            if count == 0 || count > RECURRENCE_MAX_COUNT {
                return Err(RecurrenceError::Count);
            }
        }
        Ok(())
    }
}

/// Which occurrences of a series an edit or a delete applies to, as in
/// Google Calendar's "Nur dieser Termin / Dieser und alle folgenden / Alle
/// Termine".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "calendar/")]
pub enum SeriesScope {
    /// The one occurrence: an exception of the series.
    This,
    /// This one and every later one: the series splits here.
    Following,
    /// The whole series.
    All,
}

impl SeriesScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::This => "this",
            Self::Following => "following",
            Self::All => "all",
        }
    }
}

/// `PATCH /work/events/:id/occurrences`: an edit of one occurrence of the
/// series `:id`, with its scope.
///
/// - `patch` and `expected` read as on `PATCH /work/events/:id`, against
///   the occurrence as the client saw it (its own title, time and text when
///   it has them). A stale field is a `409` `ATH-RES-003` whose
///   `details.current` is the occurrence as it is now.
/// - `this`: title, description, `when` and participants only; the links
///   and the rule belong to the series (400).
/// - `following`: the series ends before this occurrence, a new one starts
///   here with the patch (and the rest of a "N times" count).
/// - `all`: the series changes; a new `when` moves every occurrence by the
///   same step. A new time or rule drops the exceptions; a new title,
///   description or participant list drops theirs of that field.
/// - `recurrence: null` (following, all) stops repeating: what is left is
///   a single event.
///
/// The answer is the occurrence as it is now (for `following` the first
/// of the new series, for `recurrence: null` the single event).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "calendar/")]
pub struct OccurrenceEdit {
    /// The occurrence's `occurrence_start` as the window read answered it.
    pub occurrence_start: String,
    pub scope: SeriesScope,
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

/// `POST /work/events/:id/occurrences/delete`: delete one occurrence of
/// the series `:id`, this one and all after it, or the whole series
/// (`204`). The creator or a manager, as for an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "calendar/")]
pub struct OccurrenceDelete {
    pub occurrence_start: String,
    pub scope: SeriesScope,
}

/// An event of a workspace: a meeting, a review, a demo.
///
/// A window read answers single events and the occurrences of series in
/// one list. An occurrence carries `series_id` (the event the rule lives
/// on; writes go to `/work/events/:series_id/occurrences`), its
/// `occurrence_start` (the start the rule gives it, which stays when the
/// occurrence is moved: its key) and the series' `recurrence` and
/// `time_zone`. Its `id` is `<series_id>_<key>` (key `YYYYMMDDTHHMMSSZ` or
/// `YYYYMMDD`), unique in the list and stable, as Google Calendar names
/// instances; its `version` is the series'.
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
    /// The series an occurrence belongs to (absent on a single event).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub series_id: Option<String>,
    /// The occurrence's start as its rule gives it: RFC 3339 in UTC
    /// (`2026-10-27T09:00:00Z`) for a timed series, `YYYY-MM-DD` for an
    /// all-day one. The key of its exception.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub occurrence_start: Option<String>,
    /// How the series repeats (on every occurrence, for the summary).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub recurrence: Option<RecurrenceRule>,
    /// The IANA zone the series keeps its wall-clock time in, so a 09:00
    /// daily stays at 09:00 across a change of summer time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub time_zone: Option<String>,
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
    /// Makes the event a series; `when` is its first occurrence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub recurrence: Option<RecurrenceRule>,
    /// The IANA zone of the series (with `recurrence` only). Without it:
    /// the creator's zone setting, else [`SERIES_DEFAULT_TIME_ZONE`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub time_zone: Option<String>,
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
    /// A rule makes a single event a series (`PATCH /work/events/:id`) or
    /// changes a series' rule (`…/occurrences`, following or all); `null`
    /// stops repeating. A series itself is edited through `…/occurrences`.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<Object>)]
    pub recurrence: Patch<RecurrenceRule>,
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
    /// The rule as the client saw it; `null`: it did not repeat.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<Object>)]
    pub recurrence: Patch<RecurrenceRule>,
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
    fn a_rule_travels_typed_and_refuses_unknown_keys_and_bad_values() {
        let rule: RecurrenceRule = serde_json::from_value(serde_json::json!({
            "frequency": "weekly",
            "interval": 2,
            "weekdays": ["tu", "th"],
            "end": { "kind": "until", "day": "2027-03-31" }
        }))
        .unwrap_or(RecurrenceRule {
            frequency: RecurrenceFrequency::Daily,
            interval: 1,
            weekdays: Vec::new(),
            monthly: None,
            end: RecurrenceEnd::Never,
        });
        assert_eq!(rule.frequency, RecurrenceFrequency::Weekly);
        assert_eq!(rule.weekdays, [Weekday::Tu, Weekday::Th]);
        assert_eq!(
            rule.end,
            RecurrenceEnd::Until {
                day: NaiveDate::from_ymd_opt(2027, 3, 31).unwrap_or_default()
            }
        );
        assert!(rule.check().is_ok());
        let monthly: RecurrenceRule = serde_json::from_value(serde_json::json!({
            "frequency": "monthly", "interval": 1,
            "monthly": { "kind": "nth_weekday", "nth": -1, "weekday": "fr" },
            "end": { "kind": "count", "count": 10 }
        }))
        .unwrap_or(rule.clone());
        assert_eq!(
            monthly.monthly,
            Some(MonthlyRule::NthWeekday {
                nth: -1,
                weekday: Weekday::Fr
            })
        );
        for refused in [
            serde_json::json!({ "frequency": "hourly", "interval": 1, "end": { "kind": "never" } }),
            serde_json::json!({ "frequency": "daily", "interval": 1, "end": { "kind": "never" }, "byhour": 9 }),
            serde_json::json!({ "frequency": "daily", "interval": 1, "end": { "kind": "count", "count": 3, "x": 1 } }),
            serde_json::json!({ "frequency": "monthly", "interval": 1, "monthly": { "kind": "day_of_month", "day": 3, "nth": 1 }, "end": { "kind": "never" } }),
            serde_json::json!({ "frequency": "daily", "end": { "kind": "never" } }),
            serde_json::json!("FREQ=DAILY;INTERVAL=1"),
        ] {
            assert!(
                serde_json::from_value::<RecurrenceRule>(refused.clone()).is_err(),
                "{refused}"
            );
        }
        let base = RecurrenceRule {
            frequency: RecurrenceFrequency::Daily,
            interval: 1,
            weekdays: Vec::new(),
            monthly: None,
            end: RecurrenceEnd::Never,
        };
        let with = |f: &dyn Fn(&mut RecurrenceRule)| {
            let mut r = base.clone();
            f(&mut r);
            r.check()
        };
        assert_eq!(with(&|r| r.interval = 0), Err(RecurrenceError::Interval));
        assert_eq!(with(&|r| r.interval = 100), Err(RecurrenceError::Interval));
        assert_eq!(
            with(&|r| r.weekdays = vec![Weekday::Mo]),
            Err(RecurrenceError::WeekdaysOutsideWeekly)
        );
        assert_eq!(
            with(&|r| r.monthly = Some(MonthlyRule::DayOfMonth { day: 1 })),
            Err(RecurrenceError::MonthlyOutsideMonthly)
        );
        assert_eq!(
            with(&|r| {
                r.frequency = RecurrenceFrequency::Monthly;
                r.monthly = Some(MonthlyRule::DayOfMonth { day: 32 });
            }),
            Err(RecurrenceError::DayOfMonth)
        );
        assert_eq!(
            with(&|r| {
                r.frequency = RecurrenceFrequency::Monthly;
                r.monthly = Some(MonthlyRule::NthWeekday {
                    nth: 0,
                    weekday: Weekday::Mo,
                });
            }),
            Err(RecurrenceError::NthWeekday)
        );
        assert_eq!(
            with(&|r| r.end = RecurrenceEnd::Count { count: 1000 }),
            Err(RecurrenceError::Count)
        );
        assert!(with(&|r| {
            r.frequency = RecurrenceFrequency::Monthly;
            r.monthly = Some(MonthlyRule::DayOfMonth { day: -1 });
        })
        .is_ok());
    }

    #[test]
    fn an_occurrence_edit_names_its_scope_and_refuses_unknown_fields() {
        let edit: Option<OccurrenceEdit> = serde_json::from_value(serde_json::json!({
            "occurrence_start": "2026-10-27T09:00:00Z",
            "scope": "following",
            "patch": { "title": "Daily", "recurrence": null },
        }))
        .ok();
        let edit = edit.map(|e| (e.scope, e.patch.recurrence.is_null()));
        assert_eq!(edit, Some((SeriesScope::Following, true)));
        for refused in [
            serde_json::json!({ "occurrence_start": "x", "scope": "some" }),
            serde_json::json!({ "occurrence_start": "x", "scope": "this", "series": true }),
            serde_json::json!({ "scope": "this" }),
        ] {
            assert!(
                serde_json::from_value::<OccurrenceEdit>(refused.clone()).is_err(),
                "{refused}"
            );
        }
        assert!(serde_json::from_value::<OccurrenceDelete>(
            serde_json::json!({ "occurrence_start": "2026-10-27", "scope": "all", "x": 1 })
        )
        .is_err());
        assert_eq!(Weekday::from_index(6), Some(Weekday::Su));
        assert_eq!(Weekday::from_index(7), None);
        assert_eq!(
            RecurrenceFrequency::parse(RecurrenceFrequency::Yearly.as_str()),
            Some(RecurrenceFrequency::Yearly)
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
