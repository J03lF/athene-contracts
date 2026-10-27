//! Attachments of tickets and comments (W4-B).
//!
//! Files live in the database (`athene_attachments`, the bytes in
//! `athene_attachment_chunks` in pieces of [`ATTACHMENT_CHUNK_BYTES`]): the
//! host gives a module no data directory of its own, and its DB connector
//! takes one request of at most 64 KiB at a time, so a file is written
//! piece by piece. The limits of [`crate::limits`] (`ATTACHMENT_*`) say how
//! much a file, a ticket and a workspace may hold.
//!
//! **What a client sees.** The gateway takes a file as the raw request
//! body (`POST /api/v1/work/tickets/{id}/attachments`, the name in
//! [`ATTACHMENT_NAME_HEADER`]) and answers an [`AttachmentResponse`]; it
//! hands the content back as a download
//! (`GET …/attachments/{attachment_id}/content`, `?inline=true` only for
//! the image types [`AttachmentType::inline`] allows). The content type is
//! sniffed from the bytes, never taken from the client: a file whose bytes
//! are none of [`ATTACHMENT_TYPES`] is refused.
//!
//! **Between gateway and core** a file travels in parts of at most
//! [`crate::limits::ATTACHMENT_PART_MAX_BYTES`], base64 in JSON
//! ([`BeginAttachmentRequest`], [`AttachmentPartRequest`],
//! [`AttachmentContentResponse`]): the host's call envelope between modules
//! is JSON and takes at most 2 MiB per request.
//!
//! **Deleting** moves a file to the ticket's trash (`DELETE`); it can be
//! restored from there or removed for good (`POST …/purge`, only from the
//! trash), like an archived field definition.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::ticket_check::SecretKind;

/// The request header that carries the file name of an upload to the
/// gateway: UTF-8, percent-encoded (`encodeURIComponent`).
pub const ATTACHMENT_NAME_HEADER: &str = "x-athene-file-name";

/// The bytes of one stored piece of a file: one connector request carries
/// one piece (base64, about 43 KiB) with the statement and the token, under
/// the host's 64 KiB frame.
pub const ATTACHMENT_CHUNK_BYTES: usize = 32 * 1024;

/// What kind of file an attachment is, for the icon and the preview.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "attachments/")]
pub enum AttachmentKind {
    Image,
    Pdf,
    Text,
    Video,
    Archive,
    Document,
}

/// One allowed type: how it is recognised and served.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachmentType {
    /// The content type the download carries (sniffed, never the client's).
    pub content_type: &'static str,
    pub kind: AttachmentKind,
    /// File name extensions of this type, lower case, the first one the
    /// canonical: a name without one of them gets the canonical appended.
    pub extensions: &'static [&'static str],
    /// Whether the content may be shown inline (`?inline=true`): raster
    /// images only, never SVG, HTML, PDF or text.
    pub inline: bool,
}

/// The allowlist. Recognised by magic bytes, except text, which has none:
/// a text type needs its extension and valid UTF-8 without NUL bytes; the
/// office formats are ZIP containers recognised by their extension.
pub const ATTACHMENT_TYPES: &[AttachmentType] = &[
    AttachmentType {
        content_type: "image/png",
        kind: AttachmentKind::Image,
        extensions: &["png"],
        inline: true,
    },
    AttachmentType {
        content_type: "image/jpeg",
        kind: AttachmentKind::Image,
        extensions: &["jpg", "jpeg"],
        inline: true,
    },
    AttachmentType {
        content_type: "image/gif",
        kind: AttachmentKind::Image,
        extensions: &["gif"],
        inline: true,
    },
    AttachmentType {
        content_type: "image/webp",
        kind: AttachmentKind::Image,
        extensions: &["webp"],
        inline: true,
    },
    AttachmentType {
        content_type: "application/pdf",
        kind: AttachmentKind::Pdf,
        extensions: &["pdf"],
        inline: false,
    },
    AttachmentType {
        content_type: "video/mp4",
        kind: AttachmentKind::Video,
        extensions: &["mp4", "m4v"],
        inline: false,
    },
    AttachmentType {
        content_type: "video/quicktime",
        kind: AttachmentKind::Video,
        extensions: &["mov"],
        inline: false,
    },
    AttachmentType {
        content_type: "video/webm",
        kind: AttachmentKind::Video,
        extensions: &["webm"],
        inline: false,
    },
    AttachmentType {
        content_type: "application/zip",
        kind: AttachmentKind::Archive,
        extensions: &["zip"],
        inline: false,
    },
    AttachmentType {
        content_type: "application/gzip",
        kind: AttachmentKind::Archive,
        extensions: &["gz", "tgz"],
        inline: false,
    },
    AttachmentType {
        content_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        kind: AttachmentKind::Document,
        extensions: &["docx"],
        inline: false,
    },
    AttachmentType {
        content_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        kind: AttachmentKind::Document,
        extensions: &["xlsx"],
        inline: false,
    },
    AttachmentType {
        content_type: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        kind: AttachmentKind::Document,
        extensions: &["pptx"],
        inline: false,
    },
    AttachmentType {
        content_type: "application/vnd.oasis.opendocument.text",
        kind: AttachmentKind::Document,
        extensions: &["odt"],
        inline: false,
    },
    AttachmentType {
        content_type: "application/vnd.oasis.opendocument.spreadsheet",
        kind: AttachmentKind::Document,
        extensions: &["ods"],
        inline: false,
    },
    AttachmentType {
        content_type: "text/plain",
        kind: AttachmentKind::Text,
        extensions: &["txt", "log"],
        inline: false,
    },
    AttachmentType {
        content_type: "text/markdown",
        kind: AttachmentKind::Text,
        extensions: &["md"],
        inline: false,
    },
    AttachmentType {
        content_type: "text/csv",
        kind: AttachmentKind::Text,
        extensions: &["csv"],
        inline: false,
    },
    AttachmentType {
        content_type: "application/json",
        kind: AttachmentKind::Text,
        extensions: &["json"],
        inline: false,
    },
    AttachmentType {
        content_type: "application/xml",
        kind: AttachmentKind::Text,
        extensions: &["xml"],
        inline: false,
    },
    AttachmentType {
        content_type: "application/yaml",
        kind: AttachmentKind::Text,
        extensions: &["yaml", "yml"],
        inline: false,
    },
];

impl AttachmentType {
    /// The entry of `content_type`, if it is allowed.
    pub fn of(content_type: &str) -> Option<&'static AttachmentType> {
        ATTACHMENT_TYPES
            .iter()
            .find(|t| t.content_type == content_type)
    }

    /// Whether `name` ends in one of this type's extensions.
    pub fn named_by(&self, name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        self.extensions
            .iter()
            .any(|ext| lower.ends_with(&format!(".{ext}")))
    }
}

/// Where an attachment is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "attachments/")]
pub enum AttachmentStatus {
    /// Being uploaded; visible to its uploader only, and only to finish
    /// or discard it.
    Uploading,
    Active,
    /// In the ticket's trash: restore or remove for good.
    Deleted,
}

/// An attachment of a ticket, or of one of its comments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "attachments/")]
pub struct AttachmentResponse {
    pub id: String,
    pub ticket_id: String,
    /// The comment it belongs to; `null` for one of the ticket itself.
    pub comment_id: Option<String>,
    /// Sanitised: no path, no control characters, an extension of its
    /// type.
    pub name: String,
    /// Sniffed from the bytes.
    pub content_type: String,
    pub kind: AttachmentKind,
    pub size_bytes: u64,
    /// Lower-case hex SHA-256 of the content (also the download's `ETag`).
    pub sha256: String,
    /// Pixels, for the image types whose header says so.
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// Whether `…/content?inline=true` shows it in the browser.
    pub inline: bool,
    pub status: AttachmentStatus,
    pub uploaded_by: String,
    /// RFC 3339.
    pub created_at: String,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
    /// What the secret scanner of the ticket check found in a text file
    /// (critical and warning kinds), empty otherwise. The file is kept; the
    /// client warns and offers to remove it.
    pub secret_kinds: Vec<SecretKind>,
    /// Rises with every write (rename, trash, restore).
    pub version: i64,
}

/// `GET /work/tickets/:id/attachments`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "attachments/", optional_fields)]
pub struct ListAttachmentsQuery {
    /// `true`: the trash too (`status: deleted`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_deleted: Option<bool>,
}

/// `GET /work/tickets/:id/attachments`: the ticket's and its comments'
/// attachments, oldest first, with what the workspace holds in all.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "attachments/")]
pub struct AttachmentListResponse {
    pub attachments: Vec<AttachmentResponse>,
    /// Bytes the workspace stores (the trash and unfinished uploads
    /// included) and its limit.
    pub workspace_used_bytes: u64,
    pub workspace_max_bytes: u64,
}

/// The first part of an upload, from the gateway to the core:
/// `POST /work/tickets/:id/attachments` and
/// `POST /work/tickets/:id/comments/:comment_id/attachments`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "attachments/")]
pub struct BeginAttachmentRequest {
    /// As the client named it; the core sanitises it.
    pub name: String,
    /// The whole file's size.
    pub size_bytes: u64,
    /// Lower-case hex SHA-256 of the whole file; checked when the last part
    /// is in.
    pub sha256: String,
    /// The first part (all of it for a small file), base64.
    pub content_base64: String,
}

/// A further part of an upload:
/// `POST /work/tickets/:id/attachments/:attachment_id/parts`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "attachments/")]
pub struct AttachmentPartRequest {
    /// Where the part starts in the file: what the answer before said
    /// (`next_offset`).
    pub offset: u64,
    pub content_base64: String,
}

/// The core's answer to an upload part: the attachment (`uploading` until
/// the last part is in, then `active`) and where the next part starts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "attachments/")]
pub struct AttachmentUploadResponse {
    pub attachment: AttachmentResponse,
    /// `null` once the file is complete.
    pub next_offset: Option<u64>,
}

/// `GET /work/tickets/:id/attachments/:attachment_id/content` on the core:
/// one part of the content, base64, with what a download needs. The
/// gateway turns the parts into the download.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "attachments/")]
pub struct AttachmentContentResponse {
    pub name: String,
    pub content_type: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub inline: bool,
    pub offset: u64,
    pub content_base64: String,
    /// `null` on the last part.
    pub next_offset: Option<u64>,
}

/// `GET …/content`: `inline=true` for an image shown in the page; the
/// core's `offset` (a multiple of the part size) is the gateway's.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "attachments/", optional_fields)]
pub struct AttachmentContentQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
}

/// The base of a rename.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "attachments/")]
pub struct AttachmentExpected {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
}

/// `PATCH /work/tickets/:id/attachments/:attachment_id`: a new name. The
/// extension follows the type (a name without it gets it appended).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "attachments/")]
pub struct RenameAttachmentRequest {
    pub name: String,
    /// The name the client saw: `409 ATH-RES-003` (`field: name`) when it
    /// moved in between.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected: Option<AttachmentExpected>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_type_is_listed_once_with_an_extension_and_only_images_inline() {
        let mut seen = std::collections::HashSet::new();
        for t in ATTACHMENT_TYPES {
            assert!(seen.insert(t.content_type), "{} twice", t.content_type);
            assert!(!t.extensions.is_empty(), "{}", t.content_type);
            assert_eq!(
                t.inline,
                t.kind == AttachmentKind::Image,
                "{}",
                t.content_type
            );
            assert!(!t.content_type.contains("svg") && !t.content_type.contains("html"));
        }
        assert!(AttachmentType::of("image/png").is_some_and(|t| t.named_by("Shot.PNG")));
        assert!(AttachmentType::of("image/svg+xml").is_none());
    }

    #[test]
    fn the_bodies_reject_unknown_fields() {
        let part = serde_json::json!({"offset": 0, "content_base64": "", "name": "x"});
        assert!(serde_json::from_value::<AttachmentPartRequest>(part).is_err());
        let rename = serde_json::json!({"name": "a.png", "expected": {"name": "b.png"}});
        assert!(serde_json::from_value::<RenameAttachmentRequest>(rename).is_ok());
    }
}
