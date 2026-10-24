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
}
