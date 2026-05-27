//! Activity feed DTOs.
//!
//! The activity feed is UI-facing (separate from the security audit log).
//! See `ActivityRepository` in the athene crate for the persistence layer.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct ActivityEventResponse {
    pub id: String,
    pub workspace_id: String,
    pub actor_id: Option<String>,
    /// Verb (e.g., `ticket_created`, `ticket_status_changed`, `sprint_started`).
    pub action: String,
    /// `ticket`, `project`, `sprint`, `milestone`, ...
    pub target_type: String,
    pub target_id: String,
    pub detail: Option<String>,
    pub metadata: Option<JsonValue>,
    /// ISO-8601 (RFC 3339).
    pub created_at: String,
}

pub type ActivityListResponse = crate::ListResponse<ActivityEventResponse>;
