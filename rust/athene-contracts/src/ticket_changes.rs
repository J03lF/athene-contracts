//! Which tickets of a workspace changed (B5, W7-P): so open lists, boards,
//! counts and overviews stay fresh when someone else writes, without
//! reading everything again. The TypeScript in `src/generated/work/` is
//! generated from these types (see `tests/ts_bindings.rs`).
//!
//! **The core** answers what changed since a cursor:
//! `GET /work/workspaces/{id}/ticket-changes?since=<cursor>` →
//! `{ "data": TicketChangesResponse }`. Without `since` it names no
//! change and only hands out the cursor of "now", the start of a watch.
//! Members only (`403` otherwise), every project of the workspace.
//!
//! **The gateway** turns that into a live stream for the browser:
//! `GET /work/workspaces/{id}/tickets/stream?token=<stream ticket>`
//! (server-sent events, a stream ticket of `POST /auth/stream-ticket` is
//! required, the per-user stream limits apply). It asks the core every few
//! seconds **as the user who opened it**, so membership is judged on every
//! poll, and sends:
//! - [`TICKET_STREAM_READY`] once, when the watch has its cursor;
//! - [`TICKET_STREAM_CHANGED`] with a [`TicketChangedEvent`] whenever
//!   tickets changed (created, edited, moved, deleted);
//! - [`TICKET_STREAM_RESYNC`] when it may have missed changes (the core
//!   was away, or more changed than one poll carries): read the sets
//!   again;
//! - [`TICKET_STREAM_CLOSED`] with a [`TicketStreamClosed`] right before
//!   it ends on its own: the sign-in expired (ask for a new stream ticket
//!   and open again) or the workspace is no longer the user's (stop).
//!
//! A change names the ticket and its version, never its content: a client
//! that holds the ticket at an older version reads it (or its set) again,
//! one that holds this version ignores it. A change may come twice
//! (polls overlap by a moment so none slips between two); the version
//! makes the second one a no-op.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

/// The most changes one answer carries; `more` says whether others wait.
pub const TICKET_CHANGES_MAX: usize = 200;

/// How far each poll looks back before its cursor, in milliseconds: a
/// write whose transaction began before the last poll and committed after
/// it still has an earlier timestamp.
pub const TICKET_CHANGES_OVERLAP_MS: i64 = 3_000;

/// The longest cursor the core reads.
pub const TICKET_CHANGES_CURSOR_MAX_CHARS: usize = 64;

/// SSE event names of the gateway's ticket stream.
pub const TICKET_STREAM_READY: &str = "ready";
pub const TICKET_STREAM_CHANGED: &str = "ticket_changed";
pub const TICKET_STREAM_RESYNC: &str = "resync";
pub const TICKET_STREAM_CLOSED: &str = "closed";

/// `GET /work/workspaces/{id}/ticket-changes`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "work/", optional_fields)]
pub struct TicketChangesQuery {
    /// `cursor` of the answer before; absent: start watching now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
}

/// One ticket that changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "work/")]
pub struct TicketChange {
    pub ticket_id: String,
    /// Its project now; `null` for a deleted ticket.
    pub project_id: Option<String>,
    /// Its version now; `null` for a deleted ticket.
    pub version: Option<i64>,
    /// The ticket is gone (with its subtasks, which change on their own).
    pub deleted: bool,
    /// RFC 3339: when it changed.
    pub at: String,
}

/// `GET /work/workspaces/{id}/ticket-changes`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "work/")]
pub struct TicketChangesResponse {
    /// Oldest first.
    pub changes: Vec<TicketChange>,
    /// Opaque; the `since` of the next ask.
    pub cursor: String,
    /// More changes than [`TICKET_CHANGES_MAX`] were waiting: ask again
    /// at once with `cursor`.
    pub more: bool,
}

/// `ticket_changed` of the gateway's stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "work/")]
pub struct TicketChangedEvent {
    pub workspace_id: String,
    pub changes: Vec<TicketChange>,
}

/// Why the gateway's ticket stream ends on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "work/")]
pub enum TicketStreamCloseReason {
    /// The sign-in behind the stream ticket is no longer valid: get a new
    /// stream ticket (after refreshing the session) and open again.
    Expired,
    /// The workspace is not the user's (any more): do not reopen.
    Forbidden,
}

/// `closed` of the gateway's stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "work/")]
pub struct TicketStreamClosed {
    pub reason: TicketStreamCloseReason,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_close_reason_is_snake_case() {
        let closed = TicketStreamClosed {
            reason: TicketStreamCloseReason::Expired,
        };
        assert_eq!(
            serde_json::to_value(closed).unwrap_or_default(),
            serde_json::json!({ "reason": "expired" })
        );
    }
}
