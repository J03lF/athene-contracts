//! Ticket-type contracts: the configurable kinds of ticket per workspace.
//! The gateway parses request bodies with these types before forwarding;
//! the TypeScript in `src/generated/ticket_types/` is generated from them
//! (see `tests/ts_bindings.rs`), and `src/payloads/ticket-types.ts`
//! carries the full rule set (keys, default type, archive vs delete,
//! permissions). Request bodies refuse unknown fields.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::Patch;

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

/// A Lucide icon name a type may use: [`TICKET_TYPE_ICONS`] as a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export_to = "ticket_types/")]
pub enum TicketTypeIcon {
    SquareCheck,
    Bug,
    Sparkles,
    LifeBuoy,
    Layers,
    CircleDot,
    Flag,
    Zap,
    Star,
    BookOpen,
    Wrench,
    Shield,
    Rocket,
    Lightbulb,
    MessageCircle,
    FileText,
    Target,
    ListChecks,
    GitBranch,
    TriangleAlert,
    Palette,
    Package,
    Megaphone,
    FlaskConical,
}

/// A colour token (`--blue`, `--blue-tint`, …), never hex:
/// [`TICKET_TYPE_COLORS`] as a type. Workspaces use the same tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "ticket_types/")]
pub enum TicketTypeColor {
    Neutral,
    Blue,
    Green,
    Amber,
    Red,
    Purple,
    Pink,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "ticket_types/")]
pub struct TicketTypeResponse {
    pub id: String,
    pub workspace_id: String,
    /// Stable; what tickets store in `ticket_type`.
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    /// Markdown prefilled into a new ticket's empty description (≤ 8 KB);
    /// `null` = none.
    #[serde(default)]
    pub description_template: Option<String>,
    #[ts(as = "TicketTypeIcon")]
    pub icon: String,
    #[ts(as = "TicketTypeColor")]
    pub color: String,
    pub position: i32,
    pub is_default: bool,
    /// 0 = ordinary ticket, 1 = epic-like parent.
    #[ts(type = "0 | 1")]
    pub hierarchy_level: i32,
    /// `null` while live.
    pub archived_at: Option<String>,
    /// Tickets of the workspace that carry this type.
    pub ticket_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

/// `POST /work/workspaces/:id/ticket-types`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "ticket_types/")]
pub struct CreateTicketTypeRequest {
    pub name: String,
    /// Absent = derived from the name (`"Change request"` →
    /// `change_request`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_template: Option<String>,
    /// Default `circle-dot`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<TicketTypeIcon>")]
    pub icon: Option<String>,
    /// Default `neutral`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<TicketTypeColor>")]
    pub color: Option<String>,
    /// Default 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "0 | 1")]
    pub hierarchy_level: Option<i32>,
    /// `true` makes it the workspace default (the previous one stops being
    /// it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub is_default: Option<bool>,
}

/// `PATCH /work/ticket-types/:id`. An absent key leaves that part as it
/// is; `null` clears `description` or `description_template`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "ticket_types/")]
pub struct UpdateTicketTypeRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub description: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub description_template: Patch<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<TicketTypeIcon>")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<TicketTypeColor>")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "0 | 1")]
    pub hierarchy_level: Option<i32>,
    /// Only `true` is meaningful: make this the default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub is_default: Option<bool>,
    /// `true` archives, `false` restores.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub archived: Option<bool>,
}

/// `POST /work/workspaces/:id/ticket-types/reorder`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "ticket_types/")]
pub struct ReorderTicketTypesRequest {
    /// Every live and archived type id of the workspace, in the new order.
    pub ordered_ids: Vec<String>,
}

/// Query of `GET /work/workspaces/:id/ticket-types`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema, TS)]
#[ts(export_to = "ticket_types/", optional_fields)]
pub struct ListTicketTypesQuery {
    /// Include archived types (settings page). Default: live types only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_archived: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_update_tells_absent_from_null() {
        let req: UpdateTicketTypeRequest =
            serde_json::from_value(json!({ "description": null, "name": "Bug" })).expect("patch");
        assert_eq!(req.description, Patch::Null);
        assert_eq!(req.description_template, Patch::Absent);
        assert_eq!(
            serde_json::to_value(&req).expect("serialise"),
            json!({ "name": "Bug", "description": null })
        );
        assert!(
            serde_json::from_value::<UpdateTicketTypeRequest>(json!({ "colour": "red" })).is_err()
        );
    }

    #[test]
    fn the_icon_and_colour_types_name_the_lists() {
        for icon in TICKET_TYPE_ICONS {
            let parsed: TicketTypeIcon = serde_json::from_value(json!(icon)).expect("icon");
            assert_eq!(serde_json::to_value(parsed).expect("icon"), json!(icon));
        }
        for color in TICKET_TYPE_COLORS {
            let parsed: TicketTypeColor = serde_json::from_value(json!(color)).expect("colour");
            assert_eq!(serde_json::to_value(parsed).expect("colour"), json!(color));
        }
    }
}
