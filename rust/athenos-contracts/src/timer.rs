//! The work timer (W6-B): one running or paused timer per person, kept by
//! the server, so it is the same on every device and in every tab. The
//! TypeScript in `src/generated/timer/` is generated from these types (see
//! `tests/ts_bindings.rs`). Request bodies refuse unknown fields.
//!
//! - `GET  /work/timer`: the caller's timer, or `null` ([`TimerStateResponse`]).
//! - `POST /work/timer/start`: start on a ticket ([`StartTimerRequest`]); on
//!   the ticket it already holds it resumes.
//! - `POST /work/timer/pause`, `…/resume`, `…/discard`: [`TimerWriteRequest`].
//! - `POST /work/timer/switch`: move to another ticket and say what becomes
//!   of the time counted so far ([`SwitchTimerRequest`]).
//! - `POST /work/timer/finish`: book the time as a work log and clear the
//!   timer in one statement; optionally put the ticket in a done status
//!   through the normal status write ([`FinishTimerRequest`]).
//!
//! Every write answers [`TimerAnswer`], wrapped as `{ "data": … }`.
//!
//! **Time comes from the server's clock.** A timer holds the seconds it
//! counted before the current run (`base_seconds`) and when that run began
//! (`started_at`, `null` while paused); the counted time is `base_seconds +
//! (now - started_at)`. Every answer carries `server_now`, so a client can
//! tick locally from `started_at` and correct for its own clock's skew.
//!
//! **Counted time is never dropped silently.** `start` on another ticket,
//! and `switch` without `previous`, answer `409` `ATH-RES-002` with
//! `details.reason` [`TIMER_HAS_TIME_REASON`] and `details.timer` while the
//! timer holds time; the client asks and sends `previous: book` or
//! `discard`.
//!
//! **Writes carry `expected`** ([`TimerExpected`]): the version the client
//! saw, or `null` for "no timer". A stale one is `409` `ATH-RES-003` with
//! `details.current` (a [`TimerStateResponse`]) and the field `version`.
//!
//! **Who:** only one's own timer. Starting and booking need `time.log` on
//! the ticket (and so a readable ticket); the ticket's workspace is the
//! timer's. While the platform switches the module off
//! ([`TIME_TRACKING_FLAG`]) every route answers `403` `ATH-FEAT-001`.
//!
//! The timer itself is no activity; a booking writes the work log's line
//! (`ticket_work_logged`) as booking by hand does.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::status_rules::{TicketWarning, WipOverride};
use crate::work::{TicketResponse, WorkLogResponse};

/// The platform flag that switches time tracking off (`module.*` flags are
/// on while no row says otherwise).
pub const TIME_TRACKING_FLAG: &str = "module.time_tracking";

/// `details.reason` of the `409` a start or switch answers while the timer
/// holds time on another ticket.
pub const TIMER_HAS_TIME_REASON: &str = "timer_has_time";

/// The most a timer may carry in: a day and a half, the most one work log
/// books (`36 h`).
pub const TIMER_MAX_SECONDS: i64 = 36 * 60 * 60;

/// The person's timer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "timer/")]
pub struct TimerResponse {
    pub ticket_id: String,
    /// The ticket's workspace: its statuses are the ones "done" picks from.
    pub workspace_id: String,
    /// `WEB-42`, while the person can still read the ticket; else `null`.
    pub ticket_key: Option<String>,
    /// The ticket's title as it is now; `null` like `ticket_key`.
    pub ticket_title: Option<String>,
    /// Seconds counted before the current run.
    pub base_seconds: i64,
    /// RFC 3339: when the current run began; `null` while paused.
    pub started_at: Option<String>,
    /// Seconds counted at `server_now` of the answer: `base_seconds` plus
    /// the current run.
    pub elapsed_seconds: i64,
    pub note: Option<String>,
    /// Moves with every write; the base of the next one.
    pub version: i64,
    /// RFC 3339.
    pub updated_at: String,
}

/// `GET /work/timer`: the timer, or `null`, and the server's clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "timer/")]
pub struct TimerStateResponse {
    pub timer: Option<TimerResponse>,
    /// RFC 3339, the server's clock when it answered.
    pub server_now: String,
}

/// The base of a timer write: the version the client saw, `null` for "I
/// saw no timer".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "timer/")]
pub struct TimerExpected {
    pub version: Option<i64>,
}

/// Time counted somewhere else, carried into a fresh timer: a timer the
/// browser kept before the server did (uploaded once), or a discarded one
/// put back (undo). At most [`TIMER_MAX_SECONDS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "timer/")]
pub struct TimerCarry {
    pub seconds: i64,
    /// Whether it keeps running (from now on, by the server's clock).
    pub running: bool,
}

/// `POST /work/timer/start`.
///
/// - No timer: it starts on the ticket (with `carry`, from that time).
/// - On this ticket already: a paused one resumes (`carry` is ignored).
/// - On another ticket without time: it moves here.
/// - On another ticket with time: `409` [`TIMER_HAS_TIME_REASON`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "timer/")]
pub struct StartTimerRequest {
    pub ticket_id: String,
    /// Absent: not checked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected: Option<TimerExpected>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub carry: Option<TimerCarry>,
}

/// What becomes of the time counted on the ticket the timer leaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "timer/")]
pub enum TimerPrevious {
    /// Booked as a work log on that ticket (in the same statement).
    Book,
    /// Thrown away (the answer's `discarded` holds it, for an undo).
    Discard,
}

/// `POST /work/timer/switch`: the timer moves to `ticket_id`, running.
/// Without `previous` the time held is `409` [`TIMER_HAS_TIME_REASON`];
/// with `book` it goes onto the old ticket as `minutes` (default: the
/// counted time, rounded, at least one) with `note`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "timer/")]
pub struct SwitchTimerRequest {
    pub ticket_id: String,
    pub expected: TimerExpected,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub previous: Option<TimerPrevious>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub minutes: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub note: Option<String>,
    /// The new timer starts from this time instead of zero (an undo of a
    /// discarding switch puts the old timer back this way).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub carry: Option<TimerCarry>,
}

/// `POST /work/timer/pause`, `…/resume`, `…/discard`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "timer/")]
pub struct TimerWriteRequest {
    pub expected: TimerExpected,
}

/// "Ticket auf Fertig setzen" with a booking: the status write the board
/// makes (`PATCH /work/tickets/:id/status`), with its rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "timer/")]
pub struct TimerDone {
    /// A status of the category "done" the ticket's project offers.
    pub status: String,
    /// The status the client saw (as `expected.status` of a status write).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected_status: Option<String>,
    /// An owner's or admin's reason to move past a blocking WIP limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub wip_override: Option<WipOverride>,
}

/// `POST /work/timer/finish`: book and clear. `minutes` defaults to the
/// counted time (rounded, at least one); the log's `started_at` is that
/// long before now. `done` runs after the booking: the time never waits on
/// a status, and a status that is refused leaves the time booked (the
/// answer's `status` says why).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "timer/")]
pub struct FinishTimerRequest {
    pub expected: TimerExpected,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub minutes: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub done: Option<TimerDone>,
}

/// A status write the finish could not make: the error the status route
/// would have answered, with its HTTP status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "timer/")]
pub struct TimerStatusRefusal {
    /// The HTTP status the status write answered (`409`, `403`, `400`, …).
    pub status: u16,
    /// `ATH-…`.
    pub code: String,
    pub message: String,
    pub message_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "unknown")]
    #[schema(value_type = Object)]
    pub details: Option<serde_json::Value>,
}

/// What became of `done`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[ts(export_to = "timer/")]
pub enum TimerStatusOutcome {
    /// The ticket moved; `warnings` as a status write's answer has them.
    Changed {
        #[schema(value_type = Object)]
        ticket: Box<TicketResponse>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[ts(optional, as = "Option<Vec<TicketWarning>>")]
        #[schema(value_type = Vec<Object>)]
        warnings: Vec<TicketWarning>,
    },
    /// The status write refused; the time is booked all the same.
    Refused { refusal: TimerStatusRefusal },
}

/// The answer of every timer write.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "timer/")]
pub struct TimerAnswer {
    /// The timer after the write; `null` once it is booked or discarded.
    pub timer: Option<TimerResponse>,
    /// RFC 3339, the server's clock when it answered.
    pub server_now: String,
    /// The work log a finish, or a switch with `book`, wrote.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    #[schema(value_type = Object)]
    pub booked: Option<WorkLogResponse>,
    /// The timer a discard, or a switch with `discard`, threw away, as it
    /// was then (for an undo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub discarded: Option<TimerResponse>,
    /// What became of a finish's `done`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub status: Option<TimerStatusOutcome>,
}

/// `details` of the `409` [`TIMER_HAS_TIME_REASON`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "timer/")]
pub struct TimerHasTimeDetails {
    /// Always [`TIMER_HAS_TIME_REASON`].
    pub reason: String,
    /// The timer as it is now.
    pub timer: TimerResponse,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn requests_refuse_unknown_fields_and_read_their_options() {
        let start: StartTimerRequest =
            serde_json::from_str(r#"{"ticket_id":"t","carry":{"seconds":90,"running":false}}"#)
                .unwrap();
        assert_eq!(
            start.carry,
            Some(TimerCarry {
                seconds: 90,
                running: false
            })
        );
        assert_eq!(start.expected, None);
        assert!(serde_json::from_str::<StartTimerRequest>(r#"{"ticket_id":"t","x":1}"#).is_err());
        let none: TimerWriteRequest =
            serde_json::from_str(r#"{"expected":{"version":null}}"#).unwrap();
        assert_eq!(none.expected.version, None);
        assert!(serde_json::from_str::<TimerWriteRequest>(r#"{}"#).is_err());
        let switch: SwitchTimerRequest = serde_json::from_str(
            r#"{"ticket_id":"t","expected":{"version":3},"previous":"book","minutes":25}"#,
        )
        .unwrap();
        assert_eq!(switch.previous, Some(TimerPrevious::Book));
        let finish: FinishTimerRequest = serde_json::from_str(
            r#"{"expected":{"version":3},"done":{"status":"done","wip_override":{"reason":"r"}}}"#,
        )
        .unwrap();
        assert_eq!(finish.done.unwrap().status, "done");
    }

    #[test]
    fn an_outcome_is_tagged_and_an_answer_leaves_out_what_it_lacks() {
        let refused = TimerStatusOutcome::Refused {
            refusal: TimerStatusRefusal {
                status: 409,
                code: "ATH-RES-002".into(),
                message: "WIP".into(),
                message_key: "errors.conflict".into(),
                details: Some(serde_json::json!({ "reason": "wip_limit_reached" })),
            },
        };
        let json = serde_json::to_value(&refused).unwrap();
        assert_eq!(json["outcome"], "refused");
        assert_eq!(json["refusal"]["details"]["reason"], "wip_limit_reached");
        let answer = TimerAnswer {
            timer: None,
            server_now: "2026-10-27T10:00:00Z".into(),
            booked: None,
            discarded: None,
            status: None,
        };
        let json = serde_json::to_value(&answer).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "timer": null, "server_now": "2026-10-27T10:00:00Z" })
        );
    }
}
