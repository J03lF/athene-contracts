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

use crate::athene_ai::AiQualityEngine;
use crate::ticket_types::TicketTypeColor;
use crate::work::TicketPriority;
use crate::{DbEnum, Patch};

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
    /// The ticket check for this workspace (`null` = on). Off: nothing is
    /// computed, learned or stored here (`ticket_check`).
    #[serde(default)]
    pub ticket_check_enabled: Option<bool>,
    /// "Ähnliche Tickets" while writing (`null` = on).
    #[serde(default)]
    pub ticket_check_duplicates: Option<bool>,
    /// Quality hints while writing (`null` = on).
    #[serde(default)]
    pub ticket_check_quality: Option<bool>,
    /// The secret warning before saving (`null` = on).
    #[serde(default)]
    pub ticket_check_secrets: Option<bool>,
    /// "Qualitätsbewertung" (models step): `rules` alone, or `rules_model`
    /// (`null`). The model answers only while the platform has one active.
    #[serde(default)]
    pub ticket_check_quality_engine: Option<AiQualityEngine>,
    /// Triage suggestions while writing: type, priority, assignee
    /// (`null` = on).
    #[serde(default)]
    pub ticket_check_triage: Option<bool>,
    /// Display names of statuses in this workspace (custom roles C1, E6):
    /// "Bereit zum Test" for `in_review`. `null` = the product's names.
    #[serde(default)]
    pub status_labels: Option<StatusLabels>,
}

/// The longest status display name, in characters.
pub const STATUS_LABEL_MAX_CHARS: usize = 40;

/// A workspace's own names for statuses (custom roles C1). Only
/// `in_review` can be renamed for now; real custom statuses follow (C2).
/// A name is trimmed, 1–40 characters; `null` or blank = the product's
/// name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workspace/")]
pub struct StatusLabels {
    #[serde(default)]
    pub in_review: Option<String>,
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
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<bool>)]
    pub ticket_check_enabled: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<bool>)]
    pub ticket_check_duplicates: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<bool>)]
    pub ticket_check_quality: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<bool>)]
    pub ticket_check_secrets: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<AiQualityEngine>)]
    pub ticket_check_quality_engine: Patch<AiQualityEngine>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<bool>)]
    pub ticket_check_triage: Patch<bool>,
    /// Replaces the whole set of names; `null` = the product's names.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<StatusLabels>)]
    pub status_labels: Patch<StatusLabels>,
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

impl WorkspaceMemberRole {
    /// Every role, most privileged first.
    pub const ALL: [WorkspaceMemberRole; 4] = [Self::Owner, Self::Admin, Self::Member, Self::Guest];

    /// The wire and storage spelling (`owner`, …).
    pub fn as_str(&self) -> &'static str {
        self.to_db()
    }

    /// What the role may hand out: nobody grants above their own rank
    /// (`crate::policy`). Guest 0, member 1, admin 2, owner 3.
    pub const fn rank(self) -> u8 {
        match self {
            Self::Guest => 0,
            Self::Member => 1,
            Self::Admin => 2,
            Self::Owner => 3,
        }
    }
}

impl DbEnum for WorkspaceMemberRole {
    fn from_db(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("member") {
            "owner" => Ok(Self::Owner),
            "admin" => Ok(Self::Admin),
            "member" => Ok(Self::Member),
            "guest" => Ok(Self::Guest),
            other => Err(format!("invalid workspace member role '{other}'")),
        }
    }

    fn to_db(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Member => "member",
            Self::Guest => "guest",
        }
    }
}

/// One member of a workspace, with their identity resolved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "workspace/")]
pub struct WorkspaceMemberResponse {
    pub user_id: String,
    /// The tier (`owner | admin | member | guest`), from the role.
    pub role: WorkspaceMemberRole,
    /// The member's workspace role (custom roles, phase A); `null` only
    /// in memory mode.
    #[serde(default)]
    pub role_id: Option<String>,
    #[serde(default)]
    pub role_name: Option<String>,
    /// A palette token; `null` = neutral.
    #[serde(default)]
    #[ts(as = "Option<TicketTypeColor>")]
    pub role_color: Option<String>,
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
/// The server keeps only a hash of the link's secret (step 3.8a): `token`
/// is there exactly once, in the answer that minted it — `POST …/invites`
/// or `POST …/invites/:invite_id/rotate` — and `null` everywhere else, the
/// list included. A lost link is renewed (rotate), never read back.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "workspace/")]
pub struct InvitationResponse {
    pub id: String,
    pub workspace_id: String,
    /// 32 letters and digits; only in the create and rotate answers.
    #[serde(default)]
    pub token: Option<String>,
    /// Addressed to one person, or `null` for a link anybody may use.
    pub email: Option<String>,
    /// The tier the invitation grants.
    pub role: WorkspaceMemberRole,
    /// The workspace role it grants (custom roles, phase A).
    #[serde(default)]
    pub role_id: Option<String>,
    #[serde(default)]
    pub role_name: Option<String>,
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
    /// `member` when omitted. Never more than the inviter holds; an open
    /// link grants at most `member` (`policy::LINK_INVITE_MAX_ROLE`, J6),
    /// admins and owners are invited by address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub role: Option<WorkspaceMemberRole>,
    /// The workspace role to grant (its tier then applies; `role`, when
    /// also sent, must match it). Someone who does not govern hands out
    /// only roles within their own rights (403 otherwise).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub role_id: Option<String>,
    /// Omit or send `null` (or a blank string) for a link anybody may use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<i32>,
    /// RFC 3339, in the future and at most [`INVITE_MAX_DAYS`] days ahead;
    /// omitted: [`INVITE_DEFAULT_DAYS`] days from now (J4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// How long an invitation lives when the request names no expiry (J4).
pub const INVITE_DEFAULT_DAYS: i64 = 7;
/// The longest an invitation may live (J4).
pub const INVITE_MAX_DAYS: i64 = 30;

/// `PATCH /workspaces/:id/members/:user_id`.
///
/// Owners and admins only; only an owner may grant, change or remove the
/// owner role; the last owner cannot be demoted (409). `DELETE` on the same
/// path removes the member, or leaves when the id is the caller's own;
/// `POST /workspaces/:id/invites/:invite_id/revoke` revokes an invite and
/// `…/rotate` gives it a new link (the old one stops working).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workspace/")]
pub struct UpdateMemberRoleRequest {
    /// A tier: the member gets that tier's system role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub role: Option<WorkspaceMemberRole>,
    /// A workspace role (its tier then applies). One of the two is
    /// required; with both, they must agree (400).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub role_id: Option<String>,
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
    fn ticket_planning_is_a_permission_now_not_a_setting() {
        // E8: "Tickets einplanen" is a tick of the role (migration 0032).
        assert!(serde_json::from_value::<UpdateWorkspaceRequest>(
            json!({ "settings": { "ticket_planning": "managers" } })
        )
        .is_err());
        // A stored blob that still names it reads fine.
        let settings: WorkspaceSettings =
            serde_json::from_value(json!({ "ticket_planning": "managers" })).expect("settings");
        assert_eq!(settings, WorkspaceSettings::default());
    }

    #[test]
    fn a_role_change_or_invite_names_a_tier_or_a_role() {
        let req: UpdateMemberRoleRequest =
            serde_json::from_value(json!({ "role_id": "8b0c9d4e-0000-4000-8000-000000000000" }))
                .expect("role id");
        assert_eq!(req.role, None);
        let req: UpdateMemberRoleRequest =
            serde_json::from_value(json!({ "role": "guest" })).expect("tier");
        assert_eq!(req.role, Some(WorkspaceMemberRole::Guest));
        assert!(serde_json::from_value::<CreateInviteRequest>(
            json!({ "role_id": "8b0c9d4e-0000-4000-8000-000000000000" })
        )
        .is_ok());
    }

    #[test]
    fn the_assignee_default_none_is_spelled_none() {
        assert_eq!(
            serde_json::to_value(DefaultAssignee::Nobody).expect("assignee"),
            json!("none")
        );
    }
}
