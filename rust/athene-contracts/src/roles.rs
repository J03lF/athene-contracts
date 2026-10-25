//! Workspace roles (custom roles, phase A; `artifacts/roles/CONCEPT.md`):
//! named bundles of [`Permission`]s with a colour, an icon and a place in
//! the list. Every member holds exactly one (E1); the membership's tier
//! (`owner | admin | member | guest`) stays and comes from the role.
//!
//! - `GET /workspaces/:workspace_id/roles` — every member: the roles with
//!   their member counts, which of them the caller may hand out, the
//!   templates and the one-time notice for owners.
//! - `POST /workspaces/:workspace_id/roles` — a new role, empty, copied, or
//!   from a template (`roles.manage`: owners and admins).
//! - `POST /workspaces/:workspace_id/roles/presets` — several templates at
//!   once (the first-run button).
//! - `PATCH /workspaces/:workspace_id/roles/:role_id` — name, description,
//!   colour, icon, permissions; with `version` (3.7: 409 when the role
//!   changed meanwhile).
//! - `POST /workspaces/:workspace_id/roles/reorder`.
//! - `POST /workspaces/:workspace_id/roles/:role_id/delete` — its members
//!   (and open invitations) move to `replacement_role_id` in the same
//!   statement.
//! - `GET /workspaces/:workspace_id/roles/:role_id/effective` — the
//!   preview: each permission with its verdict and origin.
//! - `POST /workspaces/:workspace_id/roles/notice` — the owner's answer to
//!   the notice after migration 0032 (`restore`: members may create
//!   projects again).
//!
//! Owner and Admin are fixed: they cannot be edited, reordered or deleted.
//! "Mitglied" and "Gast" can be edited but not deleted; a guest role
//! holds only what [`GUEST_CEILING`](crate::policy::GUEST_CEILING) allows.
//! Request bodies refuse unknown fields; every response is `{ "data": … }`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::policy::{EffectivePermission, Permission, PermissionSet, RolePreset};
use crate::ticket_types::TicketTypeColor;
use crate::workspace::WorkspaceMemberRole;
use crate::Patch;

/// The longest role name, in characters (one line).
pub const ROLE_MAX_NAME_CHARS: usize = 40;
/// The longest description, in characters.
pub const ROLE_MAX_DESCRIPTION_CHARS: usize = 200;
/// The longest icon name (a lucide name: `a-z`, `0-9`, `-`).
pub const ROLE_MAX_ICON_CHARS: usize = 40;
/// The most roles a workspace may hold, system roles included.
pub const ROLE_MAX_PER_WORKSPACE: usize = 50;

/// A role of a workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "roles/")]
pub struct WorkspaceRoleResponse {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub description: Option<String>,
    /// A palette token; `null` = neutral.
    #[ts(as = "Option<TicketTypeColor>")]
    pub color: Option<String>,
    /// A lucide icon name.
    pub icon: Option<String>,
    /// `owner`, `admin`, `member`, `guest`: the four roles every workspace
    /// has. `null` for a role of its own.
    pub system_key: Option<WorkspaceMemberRole>,
    /// The template it was made from, if any.
    pub preset_key: Option<RolePreset>,
    /// What the role is under the hood: `member` or `guest` for every
    /// role but Owner and Admin.
    pub tier: WorkspaceMemberRole,
    /// The ticks, in editor order. Owner and Admin: every permission.
    #[ts(as = "Vec<Permission>")]
    #[schema(value_type = Vec<Permission>)]
    pub permissions: PermissionSet,
    /// Display order and badge priority, lowest first.
    pub position: i32,
    pub member_count: i64,
    /// Owner and Admin: nothing about them can change.
    pub locked: bool,
    /// Every role but the four system roles.
    pub deletable: bool,
    /// Whether the caller may hand this role out (invite or assign).
    pub grantable: bool,
    /// For `PATCH` (3.7).
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// What a template would create.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "roles/")]
pub struct RolePresetResponse {
    pub key: RolePreset,
    pub name: String,
    #[ts(as = "TicketTypeColor")]
    pub color: String,
    pub icon: String,
    #[ts(as = "Vec<Permission>")]
    #[schema(value_type = Vec<Permission>)]
    pub permissions: PermissionSet,
    /// A role made from this template exists already.
    pub exists: bool,
}

/// A one-time notice for the workspace's owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "roles/")]
pub enum RolesNotice {
    /// Migration 0032 took "Projekte anlegen" from the role Mitglied (E5).
    ProjectCreateOff,
}

/// `GET /workspaces/:workspace_id/roles`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "roles/")]
pub struct WorkspaceRolesResponse {
    /// By position: Owner, Admin, then the rest.
    pub roles: Vec<WorkspaceRoleResponse>,
    pub presets: Vec<RolePresetResponse>,
    /// Only for an owner, until answered.
    pub notice: Option<RolesNotice>,
    pub roles_epoch: i64,
}

/// `POST /workspaces/:workspace_id/roles`. With `preset`, the template's
/// name, colour, icon and permissions unless named here; with
/// `copy_from`, the permissions of that role.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "roles/")]
pub struct CreateRoleRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub color: Option<TicketTypeColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub icon: Option<String>,
    /// `member` (default) or `guest`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub tier: Option<WorkspaceMemberRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<Vec<Permission>>")]
    #[schema(value_type = Option<Vec<Permission>>)]
    pub permissions: Option<PermissionSet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub preset: Option<RolePreset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub copy_from: Option<String>,
}

/// `POST /workspaces/:workspace_id/roles/presets`: the templates not there
/// yet; existing ones are left alone. Answers the role list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "roles/")]
pub struct CreatePresetRolesRequest {
    pub presets: Vec<RolePreset>,
}

/// `PATCH /workspaces/:workspace_id/roles/:role_id`. Absent keys stay;
/// `null` clears the description, colour or icon.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "roles/")]
pub struct UpdateRoleRequest {
    /// The version the edit started from.
    pub version: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub description: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<TicketTypeColor>)]
    pub color: Patch<TicketTypeColor>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub icon: Patch<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<Vec<Permission>>")]
    #[schema(value_type = Option<Vec<Permission>>)]
    pub permissions: Option<PermissionSet>,
}

/// `POST /workspaces/:workspace_id/roles/reorder`: every role but Owner
/// and Admin, in the new order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "roles/")]
pub struct ReorderRolesRequest {
    pub ordered_ids: Vec<String>,
}

/// `POST /workspaces/:workspace_id/roles/:role_id/delete`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "roles/")]
pub struct DeleteRoleRequest {
    /// Where the role's members and open invitations go; a role of the
    /// same tier.
    pub replacement_role_id: String,
}

/// `POST /workspaces/:workspace_id/roles/:role_id/delete`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "roles/")]
pub struct DeleteRoleResponse {
    pub deleted_id: String,
    pub replacement_role_id: String,
    pub moved_members: i64,
}

/// `GET /workspaces/:workspace_id/roles/:role_id/effective`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "roles/")]
pub struct EffectiveRoleResponse {
    pub role_id: String,
    pub tier: WorkspaceMemberRole,
    pub entries: Vec<EffectivePermission>,
}

/// `POST /workspaces/:workspace_id/roles/notice`: an owner answers the
/// notice. `restore`: give the role Mitglied "Projekte anlegen" back.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "roles/")]
pub struct AnswerRolesNoticeRequest {
    pub restore: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_role_edit_tells_absent_from_null_and_names_its_permissions() {
        let req: UpdateRoleRequest = serde_json::from_value(json!({
            "version": 3,
            "color": null,
            "permissions": ["ticket.edit", "project.create"]
        }))
        .expect("patch");
        assert_eq!(req.color, Patch::Null);
        assert_eq!(req.description, Patch::Absent);
        assert_eq!(
            req.permissions,
            Some(PermissionSet::of(&[
                Permission::ProjectCreate,
                Permission::TicketEdit
            ]))
        );
        assert!(serde_json::from_value::<UpdateRoleRequest>(json!({
            "version": 1,
            "permissions": ["everything"]
        }))
        .is_err());
        assert!(serde_json::from_value::<UpdateRoleRequest>(json!({
            "version": 1,
            "tier": "admin"
        }))
        .is_err());
    }
}
