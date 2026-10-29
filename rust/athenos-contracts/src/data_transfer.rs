//! Import and export of a workspace's work (W13-I).
//!
//! **Import** (workspace owners and admins, into one project):
//!
//! 1. The client sends the file as the raw body to the gateway
//!    (`POST /api/v1/work/workspaces/{id}/imports?project_id=…&source=…`,
//!    the name in [`crate::attachments::ATTACHMENT_NAME_HEADER`]); the
//!    gateway hands it to the core in parts ([`BeginImportRequest`],
//!    [`ImportPartRequest`]), like an attachment. At most
//!    [`IMPORT_FILE_MAX_BYTES`].
//! 2. `GET …/imports/{import_id}/analysis` ([`ImportAnalysis`]): what the
//!    core found in the file (columns, the values of status, type,
//!    priority and people) and the mapping it suggests.
//! 3. `POST …/imports/{import_id}/preview` with an [`ImportMapping`]
//!    ([`ImportPreview`]): the first [`IMPORT_PREVIEW_ROWS`] rows as they
//!    would be created, with the problems of each, and the totals.
//! 4. `POST …/imports/{import_id}/run` with the mapping: a background job.
//!    `GET …/imports/{import_id}` ([`ImportJobResponse`]) shows its
//!    progress and, once done, its [`ImportReport`];
//!    `GET …/imports/{import_id}/errors` ([`ImportErrorsFile`]) the rows
//!    that were not created, as CSV.
//!
//! Re-running a file creates nothing twice: every created ticket is
//! remembered with its source id (the Jira key, the Trello card id, the
//! CSV's id column or, without one, the row's content) per project and
//! source, and a row whose id is known is skipped as
//! [`ImportRowAction::Duplicate`]. Attachments are not imported (v1): the
//! report lists them with their URLs. People are matched to members of
//! the workspace; nobody is invited or created.
//!
//! **Export** (workspace owners and admins): `POST …/exports`
//! ([`CreateExportRequest`]) starts a background job that writes a CSV of
//! the tickets or the full JSON ([`WORKSPACE_EXPORT_FORMAT`]) of the
//! workspace or one project; `GET …/exports/{export_id}/content` downloads
//! it until it expires ([`EXPORT_LIFETIME_HOURS`]).

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

/// The largest file an import takes, in bytes.
pub const IMPORT_FILE_MAX_BYTES: usize = 20 * 1024 * 1024;
/// Rows (tickets, cards) one import takes at most.
pub const IMPORT_ROWS_MAX: usize = 10_000;
/// Columns of a CSV at most.
pub const IMPORT_COLUMNS_MAX: usize = 500;
/// One cell, in bytes: longer cells are cut (and named in the problems).
pub const IMPORT_CELL_MAX_BYTES: usize = 64 * 1024;
/// Rows the preview shows.
pub const IMPORT_PREVIEW_ROWS: usize = 20;
/// Sample values the analysis shows per column.
pub const IMPORT_SAMPLES_PER_COLUMN: usize = 3;
/// Distinct values (statuses, types, priorities, people) the analysis
/// names per kind; the rest are counted, not named.
pub const IMPORT_VALUES_MAX: usize = 200;
/// Tickets one account may create by imports in a rolling 24 hours. An
/// import does not count against the daily ticket quota
/// (`quota_tickets_per_day`) or the per-minute limit; this is its own cap.
pub const IMPORT_TICKETS_PER_DAY: usize = 20_000;
/// Rows the report names (skipped and failed) at most; the counts are
/// always complete.
pub const IMPORT_REPORT_ROWS_MAX: usize = 2_000;
/// Attachments the report names at most.
pub const IMPORT_REPORT_ATTACHMENTS_MAX: usize = 2_000;
/// How long an export can be downloaded.
pub const EXPORT_LIFETIME_HOURS: i64 = 24;
/// The largest export file, in bytes; a larger one fails with
/// `reason: "too_large"` (export a project at a time).
pub const EXPORT_FILE_MAX_BYTES: usize = 256 * 1024 * 1024;
/// Tickets one export takes at most.
pub const EXPORT_TICKETS_MAX: usize = 50_000;
/// `format` of the full JSON export.
pub const WORKSPACE_EXPORT_FORMAT: &str = "athenos-workspace-export";
/// `version` of its shape.
pub const WORKSPACE_EXPORT_VERSION: u32 = 1;
/// Another import of the workspace is still running (`409`).
pub const IMPORT_BUSY_CODE: &str = "ATH-IMPORT-001";
/// The file is not what the source says, or cannot be read (`400`, with
/// `details.reason`).
pub const IMPORT_UNREADABLE_CODE: &str = "ATH-IMPORT-002";
/// Past the account's import cap ([`IMPORT_TICKETS_PER_DAY`], `429`).
pub const IMPORT_QUOTA_CODE: &str = "ATH-IMPORT-003";
/// The job is not in a state that allows this (`409`, `details.state`).
pub const IMPORT_STATE_CODE: &str = "ATH-IMPORT-004";
/// The export has expired or is not ready (`409`, `details.state`).
pub const EXPORT_STATE_CODE: &str = "ATH-EXPORT-001";

/// Where an import's file comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "data_transfer/")]
pub enum ImportSource {
    /// Any CSV: comma or semicolon, UTF-8 (a BOM is fine).
    Csv,
    /// A Jira Cloud CSV export ("Export → CSV (all fields)").
    Jira,
    /// A Trello board's JSON export ("Menu → Print, export and share →
    /// Export as JSON").
    Trello,
}

impl ImportSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Jira => "jira",
            Self::Trello => "trello",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "csv" => Some(Self::Csv),
            "jira" => Some(Self::Jira),
            "trello" => Some(Self::Trello),
            _ => None,
        }
    }
}

/// Where an import job is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "data_transfer/")]
pub enum ImportState {
    /// Parts of the file are still arriving.
    Uploading,
    /// The file is complete: analyse, preview, run.
    Ready,
    /// Waiting for a free place to run.
    Queued,
    Running,
    Done,
    /// Stopped by an error (`error`); what was created stays, and a run
    /// again skips it.
    Failed,
    Cancelled,
}

impl ImportState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Uploading => "uploading",
            Self::Ready => "ready",
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Some(match raw {
            "uploading" => Self::Uploading,
            "ready" => Self::Ready,
            "queued" => Self::Queued,
            "running" => Self::Running,
            "done" => Self::Done,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => return None,
        })
    }

    /// Queued or running: a second import of the workspace waits.
    pub fn is_active(self) -> bool {
        matches!(self, Self::Queued | Self::Running)
    }
}

/// What a column (or a Jira/Trello field) becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "data_transfer/")]
pub enum ImportField {
    Ignore,
    /// The id in the source: what makes a second run skip the row.
    SourceId,
    Title,
    Description,
    TicketType,
    Status,
    Priority,
    /// A person: matched by e-mail or name to a member.
    Assignee,
    Reporter,
    /// Comma- or semicolon-separated; repeated columns add up.
    Labels,
    DueDate,
    StartDate,
    StoryPoints,
    /// The source id of the parent (Jira "Parent", "Epic Link").
    Parent,
    /// A sprint of the project, by name.
    Sprint,
    /// A comment (repeated columns: one comment each).
    Comment,
    /// A custom field of the workspace (`custom_field` names its key).
    CustomField,
    /// Shown in the source note of the description ("created").
    CreatedAt,
    UpdatedAt,
    /// Listed in the report with its URL; not imported.
    Attachment,
}

/// One column and what it becomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "data_transfer/")]
pub struct ImportColumnMapping {
    /// The column's position (0-based): headers may repeat.
    pub index: u32,
    pub field: ImportField,
    /// The custom field's key, with [`ImportField::CustomField`].
    #[serde(default)]
    #[ts(optional)]
    pub custom_field: Option<String>,
}

/// A value of the source (a status, a type) and where it goes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "data_transfer/")]
pub struct ImportValueMapping {
    /// The value as the file has it.
    pub source: String,
    /// The key of the workspace's status or type; `None` with `create`
    /// makes a new one named like the source, `None` without it takes the
    /// default (the project's first status, the type "task").
    #[serde(default)]
    #[ts(optional)]
    pub target: Option<String>,
    #[serde(default)]
    pub create: bool,
}

/// A source priority and the priority it becomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "data_transfer/")]
pub struct ImportPriorityMapping {
    pub source: String,
    /// `low`, `medium`, `high` or `critical`.
    pub target: String,
}

/// A person of the source and the member they become.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "data_transfer/")]
pub struct ImportUserMapping {
    /// The name or e-mail the file has.
    pub source: String,
    /// A member of the workspace; `None`: nobody (unassigned, the
    /// importing person as reporter).
    #[serde(default)]
    #[ts(optional)]
    pub member_id: Option<String>,
}

/// How dates without a clear format are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "data_transfer/")]
pub enum ImportDateFormat {
    /// ISO (`2026-10-29`), German (`29.10.2026`) and Jira's
    /// (`29/Oct/26 3:15 PM`) are recognised by their shape.
    #[default]
    Auto,
    /// `29/10/2026` is day first.
    DayFirst,
    /// `10/29/2026` is month first.
    MonthFirst,
}

/// What a Trello checklist becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "data_transfer/")]
pub enum ChecklistMode {
    /// A task list at the end of the description (`- [x] …`).
    #[default]
    Description,
    /// One subtask per item (done items in the first done status).
    Subtasks,
}

/// The whole mapping of an import: the preview and the run take the same.
/// Its `Default` is what an empty JSON object reads as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "data_transfer/")]
pub struct ImportMapping {
    /// For CSV and Jira: every column that is not ignored. Trello's fields
    /// are fixed; its list is empty.
    #[serde(default)]
    pub columns: Vec<ImportColumnMapping>,
    #[serde(default)]
    pub statuses: Vec<ImportValueMapping>,
    #[serde(default)]
    pub types: Vec<ImportValueMapping>,
    #[serde(default)]
    pub priorities: Vec<ImportPriorityMapping>,
    #[serde(default)]
    pub users: Vec<ImportUserMapping>,
    /// Who gets a ticket whose assignee is not mapped; `None`: unassigned.
    #[serde(default)]
    #[ts(optional)]
    pub fallback_assignee_id: Option<String>,
    #[serde(default)]
    pub date_format: ImportDateFormat,
    /// Labels the workspace does not have yet are created (else dropped,
    /// with a warning).
    #[serde(default = "yes")]
    pub create_labels: bool,
    /// Sprints the project does not have yet are created (else the ticket
    /// stays outside any sprint, with a warning).
    #[serde(default)]
    pub create_sprints: bool,
    #[serde(default)]
    pub checklists: ChecklistMode,
    /// A line at the end of the description: where the ticket came from,
    /// with its original key and dates.
    #[serde(default = "yes")]
    pub source_note: bool,
    /// Comments of the source become comments (by the mapped member, else
    /// by the importing person with the original author named).
    #[serde(default = "yes")]
    pub comments: bool,
}

fn yes() -> bool {
    true
}

impl Default for ImportMapping {
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            statuses: Vec::new(),
            types: Vec::new(),
            priorities: Vec::new(),
            users: Vec::new(),
            fallback_assignee_id: None,
            date_format: ImportDateFormat::Auto,
            create_labels: true,
            create_sprints: false,
            checklists: ChecklistMode::Description,
            source_note: true,
            comments: true,
        }
    }
}

/// `POST /work/workspaces/{id}/imports/uploads` (gateway → core): the
/// first part of the file, with what the job needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "data_transfer/")]
pub struct BeginImportRequest {
    pub project_id: String,
    pub source: ImportSource,
    pub name: String,
    pub size_bytes: u64,
    /// SHA-256 of the whole file, hex.
    pub sha256: String,
    pub content_base64: String,
}

/// `POST …/imports/{import_id}/parts` (gateway → core).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "data_transfer/")]
pub struct ImportPartRequest {
    pub offset: u64,
    pub content_base64: String,
}

/// The answer to a part: the job, and where the next part starts (`None`
/// once the file is complete).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ImportUploadState {
    pub import: ImportJobResponse,
    pub next_offset: Option<u64>,
}

/// The counts of a run, as it goes.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ImportProgress {
    pub total: u32,
    pub processed: u32,
    pub created: u32,
    /// Already imported by an earlier run.
    pub duplicates: u32,
    /// Not created: invalid or refused.
    pub failed: u32,
    /// Created, with something left out (a label, a sprint, a value).
    pub warnings: u32,
}

/// One import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ImportJobResponse {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub source: ImportSource,
    pub file_name: String,
    pub size_bytes: u64,
    pub state: ImportState,
    pub created_by: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub progress: ImportProgress,
    /// Why it failed (a code), when it did.
    pub error: Option<String>,
    /// Once done (or failed after some rows).
    pub report: Option<ImportReport>,
}

/// `GET /work/workspaces/{id}/imports`: the workspace's imports, newest
/// first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ImportListResponse {
    pub imports: Vec<ImportJobResponse>,
}

/// A column the analysis found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ImportColumn {
    pub index: u32,
    pub name: String,
    /// The first non-empty values.
    pub samples: Vec<String>,
    /// Rows with a value in this column.
    pub filled: u32,
    pub suggested: ImportColumnMapping,
}

/// A distinct value of the file and how often it occurs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ImportValueCount {
    pub value: String,
    pub count: u32,
}

/// A person of the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ImportPerson {
    /// What the file names them by (an e-mail, a name, a Trello username).
    pub source: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub count: u32,
}

/// `GET …/imports/{import_id}/analysis`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ImportAnalysis {
    pub source: ImportSource,
    pub rows: u32,
    /// `","` or `";"` for a CSV.
    pub delimiter: Option<String>,
    /// CSV and Jira: the columns; Trello: none (its fields are fixed).
    pub columns: Vec<ImportColumn>,
    pub statuses: Vec<ImportValueCount>,
    pub types: Vec<ImportValueCount>,
    pub priorities: Vec<ImportValueCount>,
    pub people: Vec<ImportPerson>,
    pub labels: u32,
    pub comments: u32,
    pub checklist_items: u32,
    pub attachments: u32,
    /// The date format the dates of the file look like.
    pub date_format: ImportDateFormat,
    /// Codes of what to know (`rows_over_limit`, `cells_cut`, …).
    pub warnings: Vec<String>,
    /// What the core proposes; the client edits and sends it back.
    pub suggested: ImportMapping,
}

/// One problem of a row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ImportProblem {
    /// `error` stops the row, `warning` leaves something out.
    pub severity: ImportProblemSeverity,
    /// A stable code (`title_missing`, `date_unreadable`, `user_unknown`,
    /// `label_dropped`, `title_cut`, …) the client translates.
    pub code: String,
    /// The field it is about.
    pub field: Option<ImportField>,
    /// The value it is about, cut to 120 characters.
    pub value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "data_transfer/")]
pub enum ImportProblemSeverity {
    Error,
    Warning,
}

/// What a row will do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "data_transfer/")]
pub enum ImportRowAction {
    Create,
    /// Imported before (same source id in this project): skipped.
    Duplicate,
    /// An error: not created.
    Invalid,
}

/// One row of the preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ImportPreviewRow {
    /// 1-based, as a spreadsheet counts (the header is row 1 of a CSV, the
    /// first ticket row 2).
    pub row: u32,
    pub source_id: String,
    pub title: String,
    pub ticket_type: String,
    pub status: String,
    pub priority: String,
    pub assignee: Option<String>,
    pub labels: Vec<String>,
    pub due_date: Option<String>,
    pub story_points: Option<i32>,
    pub parent: Option<String>,
    pub comments: u32,
    pub action: ImportRowAction,
    /// The key of the ticket an earlier run created, for a duplicate.
    pub existing_key: Option<String>,
    pub problems: Vec<ImportProblem>,
}

/// The totals over every row.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ImportTotals {
    pub rows: u32,
    pub create: u32,
    pub duplicates: u32,
    pub invalid: u32,
    /// Rows to create that have warnings.
    pub with_warnings: u32,
    pub comments: u32,
    /// Statuses, types, labels and sprints the run will create.
    pub new_statuses: Vec<String>,
    pub new_types: Vec<String>,
    pub new_labels: Vec<String>,
    pub new_sprints: Vec<String>,
}

/// `POST …/imports/{import_id}/preview`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ImportPreview {
    pub rows: Vec<ImportPreviewRow>,
    pub totals: ImportTotals,
    /// How many tickets the account may still import today.
    pub quota_left: u32,
}

/// A row of the report that was not created (or created with warnings).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ImportReportRow {
    pub row: u32,
    pub source_id: String,
    pub title: String,
    pub action: ImportRowAction,
    /// The ticket's key, when one exists (created or found).
    pub ticket_key: Option<String>,
    pub problems: Vec<ImportProblem>,
}

/// An attachment of the source, not imported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ImportAttachmentRef {
    pub source_id: String,
    pub ticket_key: Option<String>,
    pub name: String,
    pub url: String,
}

/// What a run did.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ImportReport {
    pub created: u32,
    pub duplicates: u32,
    pub failed: u32,
    pub warnings: u32,
    pub comments: u32,
    pub created_statuses: Vec<String>,
    pub created_types: Vec<String>,
    pub created_labels: Vec<String>,
    pub created_sprints: Vec<String>,
    /// Skipped, failed and warned rows (at most
    /// [`IMPORT_REPORT_ROWS_MAX`]); `rows_truncated` says more existed.
    pub rows: Vec<ImportReportRow>,
    pub rows_truncated: bool,
    pub attachments: Vec<ImportAttachmentRef>,
    pub attachments_total: u32,
}

/// `GET …/imports/{import_id}/errors`: the rows that were not created
/// (and the ones with warnings), as a CSV for a spreadsheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ImportErrorsFile {
    pub file_name: String,
    pub csv: String,
}

/// What an export writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "data_transfer/")]
pub enum ExportFormat {
    /// The tickets, one row each, for a spreadsheet.
    Csv,
    /// Everything ([`WORKSPACE_EXPORT_FORMAT`]).
    Json,
}

impl ExportFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "csv" => Some(Self::Csv),
            "json" => Some(Self::Json),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "data_transfer/")]
pub enum ExportState {
    Queued,
    Running,
    Done,
    Failed,
    /// Past [`EXPORT_LIFETIME_HOURS`]: the file is gone.
    Expired,
}

impl ExportState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Expired => "expired",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Some(match raw {
            "queued" => Self::Queued,
            "running" => Self::Running,
            "done" => Self::Done,
            "failed" => Self::Failed,
            "expired" => Self::Expired,
            _ => return None,
        })
    }
}

/// `POST /work/workspaces/{id}/exports`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "data_transfer/")]
pub struct CreateExportRequest {
    pub format: ExportFormat,
    /// One project of the workspace; `None`: the whole workspace.
    #[serde(default)]
    #[ts(optional)]
    pub project_id: Option<String>,
}

/// What an export holds.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ExportCounts {
    pub projects: u32,
    pub tickets: u32,
    pub comments: u32,
    pub attachments: u32,
}

/// One export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ExportJobResponse {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub format: ExportFormat,
    pub state: ExportState,
    pub file_name: String,
    pub size_bytes: u64,
    pub created_by: String,
    pub created_at: String,
    pub finished_at: Option<String>,
    pub expires_at: String,
    pub error: Option<String>,
    pub counts: ExportCounts,
}

/// `GET /work/workspaces/{id}/exports`: the workspace's exports of the
/// last [`EXPORT_LIFETIME_HOURS`] (the expired ones a while longer, as
/// such), newest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ExportListResponse {
    pub exports: Vec<ExportJobResponse>,
}

/// `GET …/exports/{export_id}/parts?offset=` (core → gateway): one part
/// of the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "data_transfer/")]
pub struct ExportContentResponse {
    pub name: String,
    pub content_type: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub offset: u64,
    pub content_base64: String,
    pub next_offset: Option<u64>,
}

/// A cell of a CSV a spreadsheet would read as a formula, made text: a
/// leading `=`, `+`, `-`, `@`, tab or carriage return gets a `'` in front
/// (OWASP "CSV injection"). Every CSV the core writes passes through it.
pub fn csv_safe_cell(value: &str) -> std::borrow::Cow<'_, str> {
    match value.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') => std::borrow::Cow::Owned(format!("'{value}")),
        _ => std::borrow::Cow::Borrowed(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formula_cells_become_text() {
        for raw in ["=1+2", "+49 30", "-2", "@SUM(A1)", "\tx", "\r=x"] {
            assert!(csv_safe_cell(raw).starts_with('\''), "{raw:?}");
        }
        for raw in ["", "Login", "a=b", "1-2", "ä"] {
            assert_eq!(csv_safe_cell(raw), raw);
        }
    }

    #[test]
    fn a_mapping_takes_its_defaults() {
        let mapping: ImportMapping = serde_json::from_str("{}").unwrap();
        assert_eq!(mapping, ImportMapping::default());
        assert!(mapping.create_labels && mapping.source_note && mapping.comments);
        assert!(!mapping.create_sprints);
        assert_eq!(mapping.checklists, ChecklistMode::Description);
    }

    #[test]
    fn states_and_sources_read_back() {
        for s in [ImportSource::Csv, ImportSource::Jira, ImportSource::Trello] {
            assert_eq!(ImportSource::parse(s.as_str()), Some(s));
        }
        for s in [
            ImportState::Uploading,
            ImportState::Ready,
            ImportState::Queued,
            ImportState::Running,
            ImportState::Done,
            ImportState::Failed,
            ImportState::Cancelled,
        ] {
            assert_eq!(ImportState::parse(s.as_str()), Some(s));
        }
        for s in [
            ExportState::Queued,
            ExportState::Running,
            ExportState::Done,
            ExportState::Failed,
            ExportState::Expired,
        ] {
            assert_eq!(ExportState::parse(s.as_str()), Some(s));
        }
    }
}
