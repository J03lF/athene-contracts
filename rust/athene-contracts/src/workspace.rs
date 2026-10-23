use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct WorkspaceResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub logo_url: Option<String>,
    pub owner_id: String,
    pub created_at: String,
    #[serde(default)]
    pub description: Option<String>,
    /// A ticket-type colour token, not hex.
    #[serde(default)]
    pub color: Option<String>,
    /// `{ default_project_id, default_assignee, default_priority,
    /// require_estimate, composer_properties, sprint_week_start,
    /// work_day_start, work_day_end, work_days }`, every key present
    /// (`null` = unset; the working day defaults to 08:00–18:00, Mon–Fri).
    /// See `src/payloads/workspace.ts`.
    #[serde(default)]
    pub settings: serde_json::Value,
}

/// `PATCH /workspaces/:id`. Absent = unchanged; `null` clears `description`
/// / `color`; `settings` is merged key by key (`null` unsets a key).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateWorkspaceRequest {
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
    pub color: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub settings: Option<serde_json::Map<String, serde_json::Value>>,
}

fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct InviteDetailsResponse {
    pub workspace_name: String,
    pub workspace_slug: String,
    pub role: String,
    pub email_restricted: bool,
    /// Display name of whoever created the invite; `None` when that user
    /// record is gone.
    #[serde(default)]
    pub invited_by_name: Option<String>,
    /// The address the invite is restricted to, in full; `None` for an
    /// open link. Joining is refused to any other account.
    #[serde(default)]
    pub invited_email: Option<String>,
    /// The signed-in caller is already a member; then a spent invite is
    /// answered too instead of refused.
    #[serde(default)]
    pub already_member: bool,
    /// Only when `already_member`.
    #[serde(default)]
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct JoinInviteResponse {
    pub workspace_id: String,
    pub workspace_slug: String,
    pub already_member: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct CreateWorkspaceRequest {
    pub name: String,
}

/// Role a member holds in a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceMemberRole {
    Owner,
    Admin,
    Member,
    Guest,
}

/// One member of a workspace, with their identity resolved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct WorkspaceMemberResponse {
    pub user_id: String,
    pub role: WorkspaceMemberRole,
    /// RFC3339.
    pub joined_at: String,
    /// `None` when the membership outlived the user record.
    pub display_name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InvitationStatus {
    Active,
    Used,
    Expired,
    Revoked,
}

/// An invitation to a workspace. Carries the token: creating one is about
/// a link to send, and only members may list them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct InvitationResponse {
    pub id: String,
    pub workspace_id: String,
    pub token: String,
    pub email: Option<String>,
    pub role: WorkspaceMemberRole,
    pub invited_by: String,
    pub status: InvitationStatus,
    pub max_uses: Option<i32>,
    pub use_count: i32,
    pub expires_at: Option<String>,
    pub created_at: String,
}

/// `PATCH /workspaces/:id/members/:user_id`.
///
/// Owners and admins only; only an owner may grant, change or remove the
/// owner role; the last owner cannot be demoted (409). `DELETE` on the same
/// path removes the member, or leaves when the id is the caller's own;
/// `POST /workspaces/:id/invites/:invite_id/revoke` revokes an invite.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct UpdateMemberRoleRequest {
    pub role: WorkspaceMemberRole,
}

pub type WorkspaceListResponse = crate::ListResponse<WorkspaceResponse>;
