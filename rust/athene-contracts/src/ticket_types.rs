//! Ticket-type DTOs — the configurable kinds of ticket per workspace.
//! Mirrored by hand in `src/payloads/ticket-types.ts`, which carries the
//! full rule set (keys, default type, archive vs delete, permissions).

use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;

/// Lucide icon names a type may use.
pub const TICKET_TYPE_ICONS: &[&str] = &[
    "square-check",
    "bug",
    "sparkles",
    "life-buoy",
    "layers",
    "circle-dot",
    "flag",
    "zap",
    "star",
    "book-open",
    "wrench",
    "shield",
    "rocket",
    "lightbulb",
    "message-circle",
    "file-text",
    "target",
    "list-checks",
    "git-branch",
    "triangle-alert",
    "palette",
    "package",
    "megaphone",
    "flask-conical",
];

/// Colour token names (not hex).
pub const TICKET_TYPE_COLORS: &[&str] =
    &["neutral", "blue", "green", "amber", "red", "purple", "pink"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct TicketTypeResponse {
    pub id: String,
    pub workspace_id: String,
    /// Stable; what tickets store in `ticket_type`.
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    /// Markdown the composer prefills an empty description with (≤ 8 KB).
    #[serde(default)]
    pub description_template: Option<String>,
    pub icon: String,
    pub color: String,
    pub position: i32,
    pub is_default: bool,
    /// 0 = ordinary ticket, 1 = epic-like parent.
    pub hierarchy_level: i32,
    pub archived_at: Option<String>,
    pub ticket_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct CreateTicketTypeRequest {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_template: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchy_level: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
}

/// An absent key leaves that part as it is; `null` clears `description` or
/// `description_template`.
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct UpdateTicketTypeRequest {
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
    pub description_template: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchy_level: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct ReorderTicketTypesRequest {
    pub ordered_ids: Vec<String>,
}

fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer).map(Some)
}
