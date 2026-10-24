//! Workspace contracts: workspaces, their settings, members and
//! invitations (`/workspaces/*`, `/invite/*`).
//!
//! The gateway parses request bodies with these types before forwarding,
//! and the TypeScript in `src/generated/workspace/` is generated from them
//! (see `tests/ts_bindings.rs`). Every response travels in
//! `{ "data": … }`. Request bodies refuse unknown fields.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::ticket_types::TicketTypeColor;
use crate::work::TicketPriority;
use crate::Patch;

/// A workspace, as the caller sees it: with their own role and join date.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "workspace/")]
pub struct WorkspaceResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub logo_url: Option<String>,
    pub owner_id: String,
    /// The caller's role in the workspace.
    #[ts(as = "WorkspaceMemberRole")]
    pub role: String,
    /// RFC 3339: when the caller joined.
    pub joined_at: String,
    pub created_at: String,
    /// ≤ 500 chars.
    #[serde(default)]
    pub description: Option<String>,
    /// A ticket-type colour token (`blue`, `purple`, …), never hex.
    #[serde(default)]
    #[ts(as = "Option<TicketTypeColor>")]
    pub color: Option<String>,
    /// Every key present; `null` = the product default.
    #[serde(default)]
    pub settings: WorkspaceSettings,
}

/// Who a new ticket is assigned to when the composer names nobody.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "workspace/")]
pub enum DefaultAssignee {
    /// Whoever creates the ticket.
    Creator,
    /// Nobody: the ticket starts unassigned.
    #[serde(rename = "none")]
    Nobody,
}

/// The first day of a sprint week.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "workspace/")]
pub enum WeekStart {
    Monday,
    Sunday,
}

/// A property of the ticket composer a workspace may order and hide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "workspace/")]
pub enum ComposerProperty {
    Type,
    Project,
    Status,
    Priority,
    Assignee,
    Due,
    Labels,
    Points,
    Sprint,
    Release,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "workspace/")]
pub struct ComposerPropertiesSetting {
    /// Display order; properties not listed follow in the default order.
    pub order: Vec<ComposerProperty>,
    pub hidden: Vec<ComposerProperty>,
}

/// A workspace's own defaults. Every key is present in a response; `null`
/// means the product's default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "workspace/")]
pub struct WorkspaceSettings {
    /// A project of this workspace (400 otherwise).
    #[serde(default)]
    pub default_project_id: Option<String>,
    #[serde(default)]
    pub default_assignee: Option<DefaultAssignee>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub default_priority: Option<TicketPriority>,
    /// When `true`, `POST /work/tickets` without `story_points` answers 400.
    #[serde(default)]
    pub require_estimate: Option<bool>,
    #[serde(default)]
    pub composer_properties: Option<ComposerPropertiesSetting>,
    #[serde(default)]
    pub sprint_week_start: Option<WeekStart>,
    /// `"HH:MM"` (24h), when the working day starts; `null` = 08:00.
    #[serde(default)]
    pub work_day_start: Option<String>,
    /// `"HH:MM"`, after `work_day_start` (400 otherwise); `null` = 18:00.
    #[serde(default)]
    pub work_day_end: Option<String>,
    /// ISO weekdays 1 (Mon) – 7 (Sun), ascending, non-empty; `null` =
    /// Monday to Friday.
    #[serde(default)]
    pub work_days: Option<Vec<u8>>,
}

/// The settings part of a workspace patch, merged key by key: a value sets
/// the key, `null` unsets it (back to the default), an absent key stays.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workspace/")]
pub struct WorkspaceSettingsPatch {
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub default_project_id: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<DefaultAssignee>)]
    pub default_assignee: Patch<DefaultAssignee>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub default_priority: Patch<TicketPriority>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<bool>)]
    pub require_estimate: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<ComposerPropertiesSetting>)]
    pub composer_properties: Patch<ComposerPropertiesSetting>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<WeekStart>)]
    pub sprint_week_start: Patch<WeekStart>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub work_day_start: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub work_day_end: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<Vec<u8>>)]
    pub work_days: Patch<Vec<u8>>,
}

/// `PATCH /workspaces/:id`: owners and admins (403 otherwise). An absent
/// key leaves that part as it is; `null` clears `description` / `color`;
/// `settings` is merged key by key (an unknown key or a bad value refuses
/// the whole patch). An empty patch is 400.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workspace/")]
pub struct UpdateWorkspaceRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    /// ≤ 500 chars; `null` clears it.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub description: Patch<String>,
    /// A `TicketTypeColor` token; `null` clears it.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    #[ts(as = "Patch<TicketTypeColor>")]
    pub color: Patch<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub settings: Option<WorkspaceSettingsPatch>,
}

/// `GET /invite/:token`: what an invitation leads to.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "workspace/")]
pub struct InviteDetailsResponse {
    pub workspace_name: String,
    pub workspace_slug: String,
    pub role: String,
    pub email_restricted: bool,
    /// Display name of whoever created the invite; `null` when that user
    /// record is gone.
    #[serde(default)]
    pub invited_by_name: Option<String>,
    /// The address the invite is restricted to, in full; `null` for an open
    /// link. Joining is refused to any other account.
    #[serde(default)]
    pub invited_email: Option<String>,
    /// The signed-in caller is already a member. Then a spent, revoked or
    /// expired invite is answered too (instead of 400), so the page can say
    /// "you're already in <workspace_name>".
    #[serde(default)]
    pub already_member: bool,
    /// Only when `already_member`.
    #[serde(default)]
    pub workspace_id: Option<String>,
}

/// `POST /invite/:token/join`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "workspace/")]
pub struct JoinInviteResponse {
    pub workspace_id: String,
    pub workspace_slug: String,
    pub already_member: bool,
}

/// `POST /workspaces`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workspace/")]
pub struct CreateWorkspaceRequest {
    pub name: String,
}

/// Role a member holds in a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "workspace/")]
pub enum WorkspaceMemberRole {
    Owner,
    Admin,
    Member,
    Guest,
}

/// One member of a workspace, with their identity resolved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "workspace/")]
pub struct WorkspaceMemberResponse {
    pub user_id: String,
    pub role: WorkspaceMemberRole,
    /// RFC 3339.
    pub joined_at: String,
    /// `null` when the membership outlived the user record. The row is
    /// still returned so an orphaned membership stays visible instead of
    /// vanishing.
    pub display_name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "workspace/")]
pub enum InvitationStatus {
    Active,
    Used,
    Expired,
    Revoked,
}

/// An invitation to a workspace.
///
/// Carries the `token` because the point of creating one is a link to
/// send. Only members may read the list: every live token in it is a way
/// in.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "workspace/")]
pub struct InvitationResponse {
    pub id: String,
    pub workspace_id: String,
    pub token: String,
    /// Addressed to one person, or `null` for a link anybody may use.
    pub email: Option<String>,
    pub role: WorkspaceMemberRole,
    pub invited_by: String,
    pub status: InvitationStatus,
    /// Only meaningful for a link invite; `null` means unlimited.
    pub max_uses: Option<i32>,
    pub use_count: i32,
    /// RFC 3339, or `null` when it does not expire on its own.
    pub expires_at: Option<String>,
    pub created_at: String,
}

/// `POST /workspaces/:id/invites`: owners and admins.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workspace/")]
pub struct CreateInviteRequest {
    /// `member` when omitted. Never more than the inviter holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub role: Option<WorkspaceMemberRole>,
    /// Omit or send `null` (or a blank string) for a link anybody may use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<i32>,
    /// RFC 3339.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// `PATCH /workspaces/:id/members/:user_id`.
///
/// Owners and admins only; only an owner may grant, change or remove the
/// owner role; the last owner cannot be demoted (409). `DELETE` on the same
/// path removes the member, or leaves when the id is the caller's own;
/// `POST /workspaces/:id/invites/:invite_id/revoke` revokes an invite.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workspace/")]
pub struct UpdateMemberRoleRequest {
    pub role: WorkspaceMemberRole,
}

pub type WorkspaceListResponse = crate::ListResponse<WorkspaceResponse>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_update_tells_absent_from_null_down_to_the_settings() {
        let body = json!({
            "description": null,
            "settings": { "default_priority": "high", "work_days": null }
        });
        let req: UpdateWorkspaceRequest = serde_json::from_value(body.clone()).expect("patch");
        assert_eq!(req.description, Patch::Null);
        assert_eq!(req.color, Patch::Absent);
        let settings = req.settings.clone().expect("settings");
        assert_eq!(
            settings.default_priority,
            Patch::Value(TicketPriority::High)
        );
        assert_eq!(settings.work_days, Patch::Null);
        assert_eq!(settings.work_day_start, Patch::Absent);
        assert_eq!(serde_json::to_value(&req).expect("serialise"), body);
    }

    #[test]
    fn requests_refuse_unknown_fields() {
        assert!(serde_json::from_value::<UpdateWorkspaceRequest>(
            json!({ "settings": { "default_prio": "high" } })
        )
        .is_err());
        assert!(
            serde_json::from_value::<CreateInviteRequest>(json!({ "roles": "member" })).is_err()
        );
    }

    #[test]
    fn the_assignee_default_none_is_spelled_none() {
        assert_eq!(
            serde_json::to_value(DefaultAssignee::Nobody).expect("assignee"),
            json!("none")
        );
    }
}
