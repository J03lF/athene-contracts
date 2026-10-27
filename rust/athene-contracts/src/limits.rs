//! Length limits at the domain's edges, one source for the core and the web
//! (BE-16, Q-051, step 3.10).
//!
//! The core's entities check against these; `tests/ts_bindings.rs` renders
//! them into `src/generated/limits/limits.ts` for the web, so a form and the
//! server cannot disagree about a number. Lengths are Unicode scalar values
//! (`chars().count()`, what a person counts), except where a name says
//! bytes: those bound what has to fit through the host's connector.
//!
//! Limits that already lived with their own contracts stay there:
//! [`crate::docs`] (`DOC_MAX_*`), [`crate::teams`] (`TEAM_*`),
//! [`crate::roles`] (`ROLE_*`), [`crate::calendar`] (`EVENT_*`).

/// A ticket title, after trimming.
pub const TICKET_TITLE_MIN_CHARS: usize = 3;
pub const TICKET_TITLE_MAX_CHARS: usize = 160;

/// A ticket description, in UTF-8 bytes. It travels as one parameter of a
/// single connector request, whose frame the host caps at 64 KiB
/// (`fenrir_module_kit::CONNECTOR_MAX_REQUEST_FRAME`) together with the
/// statement and every other parameter; 32 KiB leaves room for JSON
/// escaping and the largest ticket write (measured in the core's
/// `ticket_description_fits_the_connector_frame`). A doc page, which holds
/// nothing else, may hold 48 KiB.
pub const TICKET_DESCRIPTION_MAX_BYTES: usize = 32 * 1024;

/// A comment, after trimming. Characters, not bytes (BE-16: "4000" used to
/// be bytes, so an umlaut counted twice).
pub const COMMENT_MIN_CHARS: usize = 2;
pub const COMMENT_MAX_CHARS: usize = 4000;

/// The note of a time booking.
pub const WORK_LOG_NOTE_MAX_CHARS: usize = 500;

/// A workspace's name.
pub const WORKSPACE_NAME_MAX_CHARS: usize = 100;

/// A sprint's name.
pub const SPRINT_NAME_MAX_CHARS: usize = 80;

/// A release's name (stored as a milestone).
pub const RELEASE_NAME_MAX_CHARS: usize = 80;

/// A label's name.
pub const LABEL_NAME_MAX_CHARS: usize = 32;

/// One attachment, in bytes (W4-B). The files live in the database, which
/// the host's connector feeds 64 KiB at a time: 10 MiB is a screenshot, a
/// log or a short screen recording, and stays a second or two of writes.
pub const ATTACHMENT_MAX_BYTES: usize = 10 * 1024 * 1024;

/// All attachments of one workspace together, in bytes: its trash and its
/// unfinished uploads included. Bounds what one workspace can put into the
/// shared database (a hundred files at the largest).
pub const ATTACHMENT_WORKSPACE_MAX_BYTES: usize = 1024 * 1024 * 1024;

/// Attachments of one ticket (its comments' included), not counting the
/// trash.
pub const ATTACHMENTS_PER_TICKET_MAX: usize = 100;

/// An attachment's name, after sanitising.
pub const ATTACHMENT_NAME_MAX_CHARS: usize = 200;

/// One part of an upload or a download between gateway and core, in
/// bytes: base64 in the JSON of the host's call envelope, which takes at
/// most 2 MiB per request. A multiple of
/// [`crate::attachments::ATTACHMENT_CHUNK_BYTES`].
pub const ATTACHMENT_PART_MAX_BYTES: usize = 1024 * 1024;

/// The JSON body of an upload part at the core: a part in base64 and the
/// fields around it.
pub const ATTACHMENT_PART_BODY_MAX_BYTES: usize = ATTACHMENT_PART_MAX_BYTES / 3 * 4 + 64 * 1024;
