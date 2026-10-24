//! Docs contracts: a workspace's markdown pages and the collections that
//! group them. The TypeScript in `src/generated/docs/` is generated from
//! them (see `tests/ts_bindings.rs`). Request bodies refuse unknown fields.
//!
//! Every response is wrapped as `{ "data": … }`; the page list adds
//! `meta: { count, total }`. Ids and timestamps travel as strings (UUID,
//! RFC 3339).
//!
//! Server rules: title 1..200 chars on one line; body at most 48 KB of
//! UTF-8 (and at most 56 KB once JSON-escaped); the slug is generated from
//! the title at creation, unique per workspace (`-2`, `-3`, … on a clash)
//! and never changes on rename. Deleting a page archives it; deleting a
//! collection leaves its pages unfiled.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use validator::Validate;

use crate::edit::TextHash;
use crate::Patch;

/// The most a page body may hold, in UTF-8 bytes.
pub const DOC_MAX_BODY_BYTES: usize = 48 * 1024;
pub const DOC_MAX_TITLE_CHARS: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "docs/")]
pub struct DocCollectionResponse {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub description: Option<String>,
    /// A short token or emoji the client renders; free text, ≤ 32 chars.
    pub icon: Option<String>,
    pub position: i32,
    /// Live (not archived) pages filed in this collection.
    pub doc_count: usize,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

/// `POST /work/workspaces/:id/doc-collections`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, ToSchema, Validate, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "docs/")]
pub struct CreateDocCollectionRequest {
    /// 1..80 chars.
    #[validate(length(min = 1, max = 80))]
    pub name: String,
    /// ≤ 500 chars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 500))]
    pub description: Option<String>,
    /// ≤ 32 chars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 32))]
    pub icon: Option<String>,
}

/// `PATCH /work/doc-collections/:id`. An absent key leaves that part as
/// it is; `null` clears it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "docs/")]
pub struct UpdateDocCollectionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub description: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub icon: Patch<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub position: Option<i32>,
}

/// A page in a list: everything but the body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "docs/")]
pub struct DocSummaryResponse {
    pub id: String,
    pub workspace_id: String,
    pub collection_id: Option<String>,
    pub project_id: Option<String>,
    pub title: String,
    pub slug: String,
    /// Plain-text preview of the body, ≤ 200 chars (markdown stripped).
    pub excerpt: String,
    pub created_by: String,
    pub updated_by: String,
    /// `None` when the editor's user record is gone.
    pub updated_by_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "docs/")]
pub struct DocListMeta {
    /// Rows in this response.
    pub count: usize,
    /// Rows matching the filter, ignoring `limit`.
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "docs/")]
pub struct DocListResponse {
    pub data: Vec<DocSummaryResponse>,
    pub meta: DocListMeta,
}

/// A whole page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "docs/")]
pub struct DocResponse {
    pub id: String,
    pub workspace_id: String,
    pub collection_id: Option<String>,
    pub project_id: Option<String>,
    pub title: String,
    pub slug: String,
    /// Markdown source. Render it sanitised — it is user input.
    pub body_md: String,
    pub created_by: String,
    pub created_by_name: Option<String>,
    pub updated_by: String,
    pub updated_by_name: Option<String>,
    /// Rises with every write to the page; orders answers, decides no
    /// conflict.
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// `POST /work/workspaces/:id/docs`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, ToSchema, Validate, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "docs/")]
pub struct CreateDocRequest {
    /// 1..200 chars, one line.
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    /// Defaults to empty. ≤ 48 KB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 49152))]
    #[ts(optional)]
    pub body_md: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

/// `PATCH /work/docs/:id`. An absent key leaves that part as it is;
/// `null` unfiles / unlinks. The slug never changes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "docs/")]
pub struct UpdateDocRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub body_md: Option<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub collection_id: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub project_id: Patch<String>,
}

impl UpdateDocRequest {
    /// `true` when the patch names no field.
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// The base of a page edit (step 3.7): per field the client changes, what it
/// saw. Title and body are long texts and are expected as the
/// [`TextHash`] of the text the client started from; `null` for a link
/// means "was not set".
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "docs/")]
pub struct DocExpected {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    #[ts(optional)]
    pub title: Option<TextHash>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    #[ts(optional)]
    pub body_md: Option<TextHash>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub collection_id: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub project_id: Patch<String>,
}

/// A page edit: `PATCH /work/docs/:id` (step 3.7). `patch` names the changes,
/// `expected` their base; a title or body someone else changed in the
/// meantime is a `409` `ATH-RES-003` whose `details.current` carries the
/// full page, so the client can show both versions and keep the user's
/// text. Stage A also accepts the bare [`UpdateDocRequest`]
/// ([`UpdateDocBody`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "docs/")]
pub struct DocEdit {
    #[serde(default, skip_serializing_if = "UpdateDocRequest::is_empty")]
    #[ts(optional, as = "Option<UpdateDocRequest>")]
    pub patch: UpdateDocRequest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expected: Option<DocExpected>,
}

/// The body of `PATCH /work/docs/:id`: a [`DocEdit`], or in stage A the bare
/// [`UpdateDocRequest`] (an edit without `expected`). A body with a
/// `patch` or `expected` key is a `DocEdit`. Not exported to TypeScript.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(untagged)]
pub enum UpdateDocBody {
    Edit(DocEdit),
    Legacy(UpdateDocRequest),
}

impl UpdateDocBody {
    /// The edit either shape asks for.
    pub fn into_edit(self) -> DocEdit {
        match self {
            Self::Edit(edit) => edit,
            Self::Legacy(patch) => DocEdit {
                patch,
                expected: None,
            },
        }
    }
}

impl<'de> Deserialize<'de> for UpdateDocBody {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let map = serde_json::Map::<String, serde_json::Value>::deserialize(deserializer)?;
        let edit = map.contains_key("patch") || map.contains_key("expected");
        let value = serde_json::Value::Object(map);
        if edit {
            DocEdit::deserialize(value).map(Self::Edit)
        } else {
            UpdateDocRequest::deserialize(value).map(Self::Legacy)
        }
        .map_err(D::Error::custom)
    }
}

/// Query of `GET /work/workspaces/:id/docs`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema, TS)]
#[ts(export_to = "docs/", optional_fields)]
pub struct DocListQuery {
    /// A collection id, or `unfiled`. Absent = every page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    /// Case-insensitive substring of title or body, ≤ 100 chars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q: Option<String>,
    /// `updated` (default, newest first) or `title`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<DocSort>")]
    pub sort: Option<String>,
    /// 1..=200, default 50.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// How a page list is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "docs/")]
pub enum DocSort {
    /// Newest change first (the default).
    Updated,
    Title,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_update_tells_absent_from_null() {
        let absent: UpdateDocRequest = serde_json::from_str(r#"{"title":"T"}"#).unwrap_or_default();
        assert_eq!(absent.collection_id, Patch::Absent);
        let cleared: UpdateDocRequest =
            serde_json::from_str(r#"{"collection_id":null}"#).unwrap_or_default();
        assert_eq!(cleared.collection_id, Patch::Null);
        assert_eq!(
            serde_json::to_string(&cleared).unwrap_or_default(),
            r#"{"collection_id":null}"#
        );
    }

    #[test]
    fn a_doc_edit_and_the_bare_patch_are_both_read() {
        let hash = "0".repeat(64);
        let edit: UpdateDocBody = serde_json::from_value(serde_json::json!({
            "patch": { "body_md": "new" },
            "expected": { "body_md": hash }
        }))
        .unwrap_or(UpdateDocBody::Legacy(UpdateDocRequest::default()));
        let UpdateDocBody::Edit(edit) = edit else {
            panic!("read as the bare patch");
        };
        assert_eq!(edit.patch.body_md.as_deref(), Some("new"));
        assert!(edit.expected.and_then(|e| e.body_md).is_some());

        let bare: UpdateDocBody = serde_json::from_value(serde_json::json!({ "title": "T" }))
            .unwrap_or(UpdateDocBody::Edit(DocEdit::default()));
        assert!(matches!(bare, UpdateDocBody::Legacy(_)));
        for refused in [
            serde_json::json!({ "patch": { "titel": "x" } }),
            serde_json::json!({ "expected": { "body_md": "short" } }),
            serde_json::json!({ "title": "x", "force": true }),
        ] {
            assert!(
                serde_json::from_value::<UpdateDocBody>(refused.clone()).is_err(),
                "{refused}"
            );
        }
    }
}
