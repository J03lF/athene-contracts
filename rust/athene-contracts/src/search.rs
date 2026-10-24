//! Ticket and doc search (roadmap step 3.9, search part):
//! `GET /work/workspaces/:workspace_id/search`.
//!
//! Full text (German stemming, German and English stop words, umlauts
//! folded) plus trigram matching on titles for typos, over the tickets of
//! one workspace the caller may list (403 for a non-member), and over its
//! live docs (title above body) as a separate list. Filters and facet
//! counts are computed by the server in the same statement.
//!
//! Highlights travel as parts (`TextPart`), never as markup: the client
//! renders `hit` parts as it likes. A secret shape in a ticket (keys,
//! tokens, passwords, URL credentials) is masked as `***` in titles and
//! snippets, and is never a search term.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::work::{TicketPriority, TicketStatus};

/// Query of `GET /work/workspaces/:workspace_id/search`. Lists are
/// comma-separated; within a list any value matches, all given filters
/// must hold.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "search/", optional_fields)]
pub struct SearchTicketsQuery {
    /// The text, at most 200 characters. A ticket key (`WEB-12`) finds that
    /// ticket first; the last word matches as a prefix while it is typed.
    /// Empty: the filtered list, most recently changed first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q: Option<String>,
    /// Project ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// Statuses (`open,in_progress,…`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Ticket-type keys (`bug,feature,…`).
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub ticket_type: Option<String>,
    /// User ids, `me` or `none` (unassigned).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    /// Label ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Hits per answer, 1–50 (default 20). Facets and `total` count all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// Doc hits per answer, 0–20 (default 5; 0: no docs). Docs are searched
    /// for a text without a status, type, assignee or label filter; a
    /// project filter keeps the pages of those projects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_limit: Option<usize>,
}

/// What a hit is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "search/")]
pub enum SearchHitKind {
    Ticket,
    Doc,
}

/// A piece of a title or snippet; `hit` marks a word the query found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "search/")]
pub struct TextPart {
    pub text: String,
    pub hit: bool,
}

/// One ticket found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "search/")]
pub struct TicketSearchHit {
    /// Always `ticket`.
    pub kind: SearchHitKind,
    pub id: String,
    pub project_id: String,
    /// `WEB-12`.
    pub key: String,
    pub sequence: i64,
    /// Secrets masked.
    pub title: String,
    /// The title in parts, the query's words marked.
    pub title_parts: Vec<TextPart>,
    /// About 180 characters of the description around the first match,
    /// `…` where cut; empty for an empty description.
    pub snippet: Vec<TextPart>,
    #[schema(value_type = String)]
    pub status: TicketStatus,
    pub ticket_type: String,
    #[schema(value_type = String)]
    pub priority: TicketPriority,
    pub assignee_id: Option<String>,
    pub updated_at: String,
    /// Relevance, for ordering only: the key match, full text and title
    /// similarity, weighted. 0 for the plain filtered list.
    pub score: f64,
    /// The query was this ticket's key.
    pub matched_key: bool,
}

/// One doc (live page) found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "search/")]
pub struct DocSearchHit {
    /// Always `doc`.
    pub kind: SearchHitKind,
    pub id: String,
    /// The page's address in its workspace.
    pub slug: String,
    pub collection_id: Option<String>,
    pub project_id: Option<String>,
    /// Secrets masked.
    pub title: String,
    pub title_parts: Vec<TextPart>,
    /// About 180 characters of the body (markdown source) around the first
    /// match.
    pub snippet: Vec<TextPart>,
    pub updated_at: String,
    pub score: f64,
}

/// How many matches have a value. `value` is `null` for "none"
/// (unassigned).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "search/")]
pub struct FacetCount {
    pub value: Option<String>,
    pub count: u64,
}

/// Counts over every match (filters applied), not only the hits returned;
/// each list by count, then value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "search/")]
pub struct TicketSearchFacets {
    pub status: Vec<FacetCount>,
    pub ticket_type: Vec<FacetCount>,
    pub assignee: Vec<FacetCount>,
    pub project: Vec<FacetCount>,
    pub label: Vec<FacetCount>,
}

/// The answer of `GET /work/workspaces/:workspace_id/search`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "search/")]
pub struct TicketSearchResponse {
    /// Best first; ties by id.
    pub hits: Vec<TicketSearchHit>,
    pub facets: TicketSearchFacets,
    /// Matches in all.
    pub total: u64,
    /// Doc hits, best first; ties by id.
    pub docs: Vec<DocSearchHit>,
    /// Doc matches in all.
    pub doc_total: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_query_refuses_unknown_parameters_and_reads_type() {
        let q: SearchTicketsQuery =
            serde_json::from_value(serde_json::json!({ "q": "login", "type": "bug,task" }))
                .unwrap();
        assert_eq!(q.ticket_type.as_deref(), Some("bug,task"));
        assert!(serde_json::from_value::<SearchTicketsQuery>(
            serde_json::json!({ "q": "x", "workspace_id": "w" })
        )
        .is_err());
    }
}
