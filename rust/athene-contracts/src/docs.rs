//! Docs DTOs — a workspace's markdown pages and the collections that group
//! them. Mirrored by hand in `src/payloads/docs.ts`.
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

use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// The most a page body may hold, in UTF-8 bytes.
pub const DOC_MAX_BODY_BYTES: usize = 48 * 1024;
pub const DOC_MAX_TITLE_CHARS: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
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

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
pub struct CreateDocCollectionRequest {
    #[validate(length(min = 1, max = 80))]
    pub name: String,
    #[validate(length(max = 500))]
    pub description: Option<String>,
    #[validate(length(max = 32))]
    pub icon: Option<String>,
}

/// An absent key leaves that part as it is; `null` clears it.
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct UpdateDocCollectionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<String>)]
    pub description: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<String>)]
    pub icon: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
}

/// A page in a list: everything but the body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct DocListMeta {
    /// Rows in this response.
    pub count: usize,
    /// Rows matching the filter, ignoring `limit`.
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct DocListResponse {
    pub data: Vec<DocSummaryResponse>,
    pub meta: DocListMeta,
}

/// A whole page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
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

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
pub struct CreateDocRequest {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    /// Defaults to empty.
    #[validate(length(max = 49152))]
    pub body_md: Option<String>,
    pub collection_id: Option<String>,
    pub project_id: Option<String>,
}

/// An absent key leaves that part as it is; `null` unfiles / unlinks.
/// The slug never changes.
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct UpdateDocRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_md: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<String>)]
    pub collection_id: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<String>)]
    pub project_id: Option<Option<String>>,
}

/// Query of `GET /work/workspaces/:id/docs`.
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct DocListQuery {
    /// A collection id, or `unfiled`. Absent = every page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    /// Case-insensitive substring of title or body, ≤ 100 chars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q: Option<String>,
    /// `updated` (default, newest first) or `title`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    /// 1..=200, default 50.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// Tells "key absent" (`None`) from "key set to null" (`Some(None)`).
fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_update_tells_absent_from_null() {
        let absent: UpdateDocRequest = serde_json::from_str(r#"{"title":"T"}"#).unwrap_or_default();
        assert_eq!(absent.collection_id, None);
        let cleared: UpdateDocRequest =
            serde_json::from_str(r#"{"collection_id":null}"#).unwrap_or_default();
        assert_eq!(cleared.collection_id, Some(None));
        assert_eq!(
            serde_json::to_string(&cleared).unwrap_or_default(),
            r#"{"collection_id":null}"#
        );
    }
}
