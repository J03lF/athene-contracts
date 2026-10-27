//! Release notes (W4-H): generated on a button press from the finished
//! tickets of a release, rule-based (no AI).
//!
//! - **Template** per workspace ([`ReleaseNoteTemplate`]): title, intro and
//!   outro (placeholders `{release}`, `{project}`, `{date}`, `{count}`),
//!   sections in order, each matching ticket types and/or labels (the first
//!   section that matches takes the ticket), a heading for what no section
//!   takes (or leave it out), and which ticket fields each variant shows.
//!   German and English defaults ([`ReleaseNoteTemplate::default_for`]).
//! - **Per release** ([`ReleaseNotesSettingsResponse`]): tickets left out (of
//!   the public notes, or of both), the order within a section, the
//!   release's own intro and outro, and a stored, editable draft per
//!   variant.
//! - **Per ticket**: an optional "Release-Notiz" used instead of the title
//!   ([`crate::limits::TICKET_RELEASE_NOTE_MAX_CHARS`]).
//! - **Variants** ([`ReleaseNotesVariant`]): `internal` shows every field and
//!   marks what the public notes leave out; `public` hides the internal
//!   fields (assignee, the app link) and the excluded tickets.
//! - **Output**: Markdown, and HTML rendered from it by an allowlist (only
//!   headings, paragraphs, lists, emphasis, code and http(s)/mailto links;
//!   everything else is text).
//!
//! "Finished" is a status of the done category (`TicketStatus::closes`).
//!
//! Routes (every response `{ "data": … }`; bodies refuse unknown fields):
//! - `GET /work/workspaces/:workspace_id/release-note-template` (members),
//!   `PUT` and `DELETE` (back to the default) with `workspace.settings.edit`.
//! - `GET /work/milestones/:id/notes?variant=` (members) — the live
//!   preview, the stored draft and the release's settings.
//! - `PATCH /work/milestones/:id/notes` ([`UpdateReleaseNotesRequest`]),
//!   `POST /work/milestones/:id/notes/generate` — `project.plan` (the right
//!   that edits the release).
//! - `GET /work/milestones/:id/notes/download?variant=&format=&live=` (the
//!   gateway) — the file as a download; the core answers it as JSON at
//!   `…/notes/file` ([`ReleaseNotesFileResponse`]).
//! - `GET` / `PUT /work/tickets/:id/release-note` — read / `ticket.edit`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::edit::TextHash;
use crate::Patch;

/// `409` `details.reason` of a generate that would overwrite a draft
/// somebody edited by hand (send `replace_edited: true` to do it anyway).
pub const DRAFT_EDITED_REASON: &str = "draft_edited";

/// The template's language: the defaults' texts, and the date format.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "release_notes/")]
pub enum ReleaseNoteLanguage {
    #[default]
    De,
    En,
}

/// Who the notes are for.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "release_notes/")]
pub enum ReleaseNotesVariant {
    /// The team: every field, the excluded tickets marked.
    #[default]
    Internal,
    /// Customers: no internal fields, no excluded tickets.
    Public,
}

impl ReleaseNotesVariant {
    pub const ALL: [Self; 2] = [Self::Internal, Self::Public];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Internal => "internal",
            Self::Public => "public",
        }
    }
}

/// A download's format.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "release_notes/")]
pub enum ReleaseNotesFormat {
    #[default]
    Markdown,
    Html,
}

/// How a section's types and labels combine.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "release_notes/")]
pub enum SectionMatch {
    /// The ticket's type is listed, or one of its labels is.
    #[default]
    Any,
    /// Both: its type is listed (or none are) and one of its labels is
    /// (or none are).
    All,
}

/// One section of the notes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "release_notes/")]
pub struct ReleaseNoteSection {
    /// Stable within the template: `^[a-z][a-z0-9_-]{0,31}$`. The per-
    /// release order names it.
    pub key: String,
    /// One line, 1–[`crate::limits::RELEASE_NOTE_HEADING_MAX_CHARS`].
    pub heading: String,
    /// Ticket type keys (`bug`, `feature`, …).
    #[serde(default)]
    pub ticket_types: Vec<String>,
    /// Label names, compared without case.
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub match_mode: SectionMatch,
}

/// Which ticket fields a variant shows beside the text (the release note,
/// else the title).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "release_notes/")]
pub struct ReleaseNoteFields {
    /// `WEB-12`.
    pub key: bool,
    /// A link built from the variant's link template (none without one).
    pub link: bool,
    /// The type's name.
    pub ticket_type: bool,
    pub labels: bool,
    /// The assignee's name; internal only (ignored for `public`).
    pub assignee: bool,
}

/// A workspace's template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "release_notes/")]
pub struct ReleaseNoteTemplate {
    pub language: ReleaseNoteLanguage,
    /// The document's heading; placeholders as in the intro.
    pub title: String,
    /// Markdown before the sections (may be empty). Placeholders:
    /// `{release}`, `{project}`, `{date}`, `{count}`.
    pub intro: String,
    /// Markdown after the sections (may be empty).
    pub outro: String,
    /// In order; at most [`crate::limits::RELEASE_NOTE_SECTIONS_MAX`].
    pub sections: Vec<ReleaseNoteSection>,
    /// The heading of the tickets no section takes; `null` leaves them out.
    pub other_heading: Option<String>,
    /// What an empty release says.
    pub empty_text: String,
    pub internal_fields: ReleaseNoteFields,
    pub public_fields: ReleaseNoteFields,
    /// `https://…` with `{key}` and/or `{id}`, e.g. the app's ticket page
    /// (`https://athene.example/app/tickets/{id}`). `null`: no links.
    pub internal_link: Option<String>,
    /// The same for the public notes (a public tracker), or `null`.
    pub public_link: Option<String>,
}

impl ReleaseNoteTemplate {
    /// The product's template in `language`.
    pub fn default_for(language: ReleaseNoteLanguage) -> Self {
        let section =
            |key: &str, heading: &str, types: &[&str], labels: &[&str]| ReleaseNoteSection {
                key: key.to_string(),
                heading: heading.to_string(),
                ticket_types: types.iter().map(|s| (*s).to_string()).collect(),
                labels: labels.iter().map(|s| (*s).to_string()).collect(),
                match_mode: SectionMatch::Any,
            };
        let internal_fields = ReleaseNoteFields {
            key: true,
            link: true,
            ticket_type: false,
            labels: false,
            assignee: true,
        };
        let public_fields = ReleaseNoteFields {
            key: false,
            link: false,
            ticket_type: false,
            labels: false,
            assignee: false,
        };
        match language {
            ReleaseNoteLanguage::De => Self {
                language,
                title: "Release {release}".to_string(),
                intro: "Das ist neu in {project} {release} ({date}).".to_string(),
                outro: String::new(),
                sections: vec![
                    section("new", "Neu", &["feature"], &["neu", "feature"]),
                    section(
                        "improved",
                        "Verbessert",
                        &["task", "support"],
                        &["verbesserung", "improvement"],
                    ),
                    section("fixed", "Behoben", &["bug"], &["bug", "fix"]),
                ],
                other_heading: Some("Weitere Änderungen".to_string()),
                empty_text: "Diese Version enthält noch keine abgeschlossenen Tickets.".to_string(),
                internal_fields,
                public_fields,
                internal_link: None,
                public_link: None,
            },
            ReleaseNoteLanguage::En => Self {
                language,
                title: "Release {release}".to_string(),
                intro: "What's new in {project} {release} ({date}).".to_string(),
                outro: String::new(),
                sections: vec![
                    section("new", "New", &["feature"], &["new", "feature"]),
                    section(
                        "improved",
                        "Improved",
                        &["task", "support"],
                        &["improvement"],
                    ),
                    section("fixed", "Fixed", &["bug"], &["bug", "fix"]),
                ],
                other_heading: Some("Other changes".to_string()),
                empty_text: "This release has no finished tickets yet.".to_string(),
                internal_fields,
                public_fields,
                internal_link: None,
                public_link: None,
            },
        }
    }
}

/// `GET /work/workspaces/:workspace_id/release-note-template`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "release_notes/")]
pub struct ReleaseNoteTemplateResponse {
    pub workspace_id: String,
    pub template: ReleaseNoteTemplate,
    /// No template stored: `template` is the German default.
    pub is_default: bool,
    /// Rises with every save; `PUT` may send it back as `expected_version`.
    pub version: i64,
    /// The caller may change it (`workspace.settings.edit`).
    pub editable: bool,
    /// The product's templates, to start over from.
    pub defaults: ReleaseNoteTemplateDefaults,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "release_notes/")]
pub struct ReleaseNoteTemplateDefaults {
    pub de: ReleaseNoteTemplate,
    pub en: ReleaseNoteTemplate,
}

/// `PUT /work/workspaces/:workspace_id/release-note-template`: the whole
/// template. With `expected_version`, a template saved meanwhile is a `409
/// ATH-RES-003` (`details.current` the template now).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "release_notes/")]
pub struct UpdateReleaseNoteTemplateRequest {
    pub template: ReleaseNoteTemplate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected_version: Option<i64>,
}

/// Where a ticket is left out.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "release_notes/")]
pub enum ExclusionScope {
    /// Not in the public notes; the internal ones mark it.
    #[default]
    Public,
    /// In neither.
    All,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "release_notes/")]
pub struct ReleaseNoteExclusion {
    pub ticket_id: String,
    #[serde(default)]
    pub scope: ExclusionScope,
}

/// The order of one section's tickets; tickets not named follow in their
/// default order (by key).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "release_notes/")]
pub struct ReleaseNoteSectionOrder {
    /// A section key, or `other`.
    pub section: String,
    pub ticket_ids: Vec<String>,
}

/// The key the tickets of no section go under.
pub const OTHER_SECTION_KEY: &str = "other";

/// One release's own settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "release_notes/")]
pub struct ReleaseNotesSettingsResponse {
    pub excluded: Vec<ReleaseNoteExclusion>,
    pub order: Vec<ReleaseNoteSectionOrder>,
    /// The release's own intro / outro; `null` = the template's.
    pub intro: Option<String>,
    pub outro: Option<String>,
    /// Rises with every write to the release's notes.
    pub version: i64,
}

/// One ticket of the notes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "release_notes/")]
pub struct ReleaseNoteEntry {
    pub ticket_id: String,
    pub key: String,
    pub title: String,
    /// The ticket's release note, if it has one.
    pub note: Option<String>,
    /// What the notes say: the note, else the title.
    pub text: String,
    pub ticket_type: String,
    pub labels: Vec<String>,
    /// Internal only (`null` in the public variant).
    pub assignee_name: Option<String>,
    /// The built link, when the variant shows links and has a template.
    pub link: Option<String>,
    /// Left out of the public notes, or of both (internal variant only).
    pub excluded: Option<ExclusionScope>,
}

/// One section with its tickets, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "release_notes/")]
pub struct ReleaseNotesSectionView {
    /// A template section's key, or [`OTHER_SECTION_KEY`].
    pub key: String,
    pub heading: String,
    pub entries: Vec<ReleaseNoteEntry>,
}

/// A stored draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "release_notes/")]
pub struct ReleaseNotesDraftResponse {
    pub variant: ReleaseNotesVariant,
    pub markdown: String,
    /// `markdown` rendered by the allowlist.
    pub html: String,
    /// SHA-256 of `markdown` (`TextHash`), the base of an edit.
    pub hash: String,
    /// Somebody changed it after it was generated.
    pub edited: bool,
    /// RFC 3339.
    pub generated_at: Option<String>,
    pub updated_at: String,
    pub updated_by: Option<String>,
}

/// `GET /work/milestones/:id/notes?variant=`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "release_notes/")]
pub struct ReleaseNotesResponse {
    pub release_id: String,
    pub project_id: String,
    pub variant: ReleaseNotesVariant,
    /// The live preview from the tickets now. Internal: excluded tickets
    /// stay, marked; `excluded_all` ones are listed in `left_out`.
    pub sections: Vec<ReleaseNotesSectionView>,
    /// Tickets left out of this variant (for the "add back" list).
    pub left_out: Vec<ReleaseNoteEntry>,
    /// Tickets of the release that are not finished yet.
    pub unfinished: usize,
    pub markdown: String,
    pub html: String,
    /// The stored draft of this variant, if there is one; downloads use it.
    pub draft: Option<ReleaseNotesDraftResponse>,
    pub settings: ReleaseNotesSettingsResponse,
    /// The caller may change them (`project.plan`).
    pub editable: bool,
}

/// The base of a release-notes edit: per part the value (texts: the
/// hash) the client saw. A part that moved meanwhile is a `409
/// ATH-RES-003` naming it, nothing is written.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "release_notes/")]
pub struct ReleaseNotesExpected {
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub intro: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub outro: Patch<String>,
    /// The draft's hash, or `null` for "there was none".
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub draft_internal: Patch<TextHash>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub draft_public: Patch<TextHash>,
    /// The order as the client saw it (the whole list).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub order: Option<Vec<ReleaseNoteSectionOrder>>,
}

/// Exclusions as a set: no conflict between two people's changes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "release_notes/")]
pub struct ExclusionOps {
    #[serde(default)]
    pub add: Vec<ReleaseNoteExclusion>,
    /// Ticket ids.
    #[serde(default)]
    pub remove: Vec<String>,
}

/// `PATCH /work/milestones/:id/notes`: only the parts present change.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "release_notes/")]
pub struct UpdateReleaseNotesRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub exclude: Option<ExclusionOps>,
    /// Replaces the order of the sections named (an empty list resets one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub order: Option<Vec<ReleaseNoteSectionOrder>>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub intro: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub outro: Patch<String>,
    /// The edited Markdown; `null` drops the draft (back to the preview).
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub draft_internal: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub draft_public: Patch<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected: Option<ReleaseNotesExpected>,
}

/// `POST /work/milestones/:id/notes/generate`: store the preview as the
/// draft. A draft edited by hand is a `409` ([`DRAFT_EDITED_REASON`])
/// unless `replace_edited`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "release_notes/")]
pub struct GenerateReleaseNotesRequest {
    /// Default: both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub variants: Option<Vec<ReleaseNotesVariant>>,
    /// `true` overwrites a draft edited by hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub replace_edited: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "release_notes/")]
pub struct GenerateReleaseNotesResponse {
    pub drafts: Vec<ReleaseNotesDraftResponse>,
}

/// `GET /work/milestones/:id/notes` and `…/download`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "release_notes/", optional_fields)]
pub struct ReleaseNotesQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<ReleaseNotesVariant>,
    /// Downloads only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<ReleaseNotesFormat>,
    /// Downloads only: `true` ignores the draft and uses the preview.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live: Option<bool>,
}

/// The core's answer to `…/notes/file`; the gateway's `…/notes/download` turns it into the
/// file (`Content-Disposition: attachment`, `nosniff`, a sandboxing CSP).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "release_notes/")]
pub struct ReleaseNotesFileResponse {
    /// ASCII, e.g. `release-1.2-public.md`.
    pub file_name: String,
    /// `text/markdown; charset=utf-8` or `text/html; charset=utf-8`.
    pub content_type: String,
    pub content: String,
}

/// `GET /work/tickets/:id/release-note`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "release_notes/")]
pub struct TicketReleaseNoteResponse {
    pub ticket_id: String,
    pub note: Option<String>,
    pub updated_at: Option<String>,
    pub updated_by: Option<String>,
}

/// `PUT /work/tickets/:id/release-note`. `note: null` (or blank) removes it.
/// With `expected`, a note changed meanwhile is a `409 ATH-RES-003`
/// (`details.current` the note now).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "release_notes/")]
pub struct UpdateTicketReleaseNoteRequest {
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected: Option<TicketReleaseNoteExpected>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "release_notes/")]
pub struct TicketReleaseNoteExpected {
    pub note: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_parse_back_and_differ_by_language() {
        for language in [ReleaseNoteLanguage::De, ReleaseNoteLanguage::En] {
            let template = ReleaseNoteTemplate::default_for(language);
            let wire = serde_json::to_value(&template).unwrap();
            let back: ReleaseNoteTemplate = serde_json::from_value(wire).unwrap();
            assert_eq!(back, template);
            assert_eq!(template.sections.len(), 3);
            assert!(!template.public_fields.assignee);
        }
        assert_ne!(
            ReleaseNoteTemplate::default_for(ReleaseNoteLanguage::De).sections[0].heading,
            ReleaseNoteTemplate::default_for(ReleaseNoteLanguage::En).sections[0].heading
        );
    }

    #[test]
    fn requests_refuse_unknown_fields_and_tell_absent_from_null() {
        let req: UpdateReleaseNotesRequest = serde_json::from_value(serde_json::json!({
            "intro": null,
            "draft_public": "# Notes",
            "exclude": { "add": [{ "ticket_id": "t1" }] },
        }))
        .unwrap();
        assert_eq!(req.intro, Patch::Null);
        assert_eq!(req.outro, Patch::Absent);
        assert_eq!(req.draft_public, Patch::Value("# Notes".to_string()));
        assert_eq!(
            req.exclude.unwrap().add[0].scope,
            ExclusionScope::Public,
            "public is the default scope"
        );
        for body in [
            serde_json::json!({ "drafts": {} }),
            serde_json::json!({ "exclude": { "add": [], "keep": [] } }),
            serde_json::json!({ "expected": { "draft_internal": "short" } }),
        ] {
            assert!(
                serde_json::from_value::<UpdateReleaseNotesRequest>(body.clone()).is_err(),
                "{body}"
            );
        }
        let mut template =
            serde_json::to_value(ReleaseNoteTemplate::default_for(ReleaseNoteLanguage::En))
                .unwrap();
        template["colour"] = serde_json::json!("red");
        assert!(serde_json::from_value::<ReleaseNoteTemplate>(template).is_err());
    }
}
