//! Teams contracts: named groups of a workspace's members. The gateway
//! parses request bodies with these types before forwarding; the
//! TypeScript in `src/generated/teams/` is generated from them (see
//! `tests/ts_bindings.rs`). Request bodies refuse unknown fields.
//!
//! Every response is wrapped as `{ "data": … }`. Ids and timestamps travel
//! as strings (UUID, RFC 3339).
//!
//! Server rules: name 1..80 chars on one line, unique per workspace
//! ignoring case; key 2..10 of `A-Z`/`0-9` starting with a letter, stored
//! upper-case, unique per workspace — suggested from the name (numbered on
//! a clash) when absent; description ≤ 500 chars; colour `#RRGGBB`. Only
//! members of the workspace can be in its teams, and leaving the workspace
//! leaves its teams. Owners and admins manage; every member reads; anyone
//! may remove themselves (leave).

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use validator::Validate;

use crate::Patch;

pub const TEAM_MAX_NAME_CHARS: usize = 80;
pub const TEAM_MAX_DESCRIPTION_CHARS: usize = 500;
pub const TEAM_MIN_KEY_CHARS: usize = 2;
pub const TEAM_MAX_KEY_CHARS: usize = 10;
/// How many members a team in a list carries along with `member_count`.
pub const TEAM_MEMBER_PREVIEW: usize = 5;
/// The most people one add-members request may name.
pub const TEAM_MAX_MEMBERS_PER_REQUEST: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "teams/")]
pub enum TeamRole {
    Lead,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "teams/")]
pub struct TeamMemberResponse {
    pub user_id: String,
    pub role: TeamRole,
    pub added_at: String,
    /// `None` when the user record is gone.
    pub display_name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "teams/")]
pub struct TeamResponse {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub key: String,
    pub description: Option<String>,
    /// `#RRGGBB`.
    pub color: Option<String>,
    /// Everyone in the team.
    pub member_count: usize,
    /// Leads first, then by when they joined. In a list, the first
    /// [`TEAM_MEMBER_PREVIEW`]; for a single team, everyone.
    pub members: Vec<TeamMemberResponse>,
    /// The caller's role in the team; `None` when they are not in it.
    pub my_role: Option<TeamRole>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "teams/")]
pub struct TeamMemberInput {
    pub user_id: String,
    /// Defaults to `member`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub role: Option<TeamRole>,
}

/// `POST /work/workspaces/:id/teams`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, ToSchema, Validate, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "teams/")]
pub struct CreateTeamRequest {
    /// 1..80 chars.
    #[validate(length(min = 1, max = 80))]
    pub name: String,
    /// Absent = suggested from the name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 2, max = 10))]
    #[ts(optional)]
    pub key: Option<String>,
    /// ≤ 500 chars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 500))]
    pub description: Option<String>,
    /// `#RRGGBB`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// People to put in the team straight away; each must be a member of
    /// the workspace.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<TeamMemberInput>,
}

/// `PATCH /work/teams/:id`. An absent key leaves that part as it is;
/// `null` clears it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "teams/")]
pub struct UpdateTeamRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub description: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub color: Patch<String>,
}

/// Either `members` (a role each) or `user_ids` sharing `role`, or both.
/// Someone already in the team keeps their role.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "teams/")]
pub struct AddTeamMembersRequest {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<TeamMemberInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user_ids: Vec<String>,
    /// For `user_ids`; defaults to `member`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub role: Option<TeamRole>,
}

/// `PATCH /work/teams/:id/members/:user_id`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "teams/")]
pub struct UpdateTeamMemberRequest {
    pub role: TeamRole,
}

/// Query of `GET /work/workspaces/:id/teams`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema, TS)]
#[ts(export_to = "teams/", optional_fields)]
pub struct TeamListQuery {
    /// A user id, or `me`: only the teams that person is in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_update_tells_absent_from_null() {
        let absent: UpdateTeamRequest = serde_json::from_str(r#"{"name":"Ops"}"#).expect("parse");
        assert_eq!(absent.color, Patch::Absent);
        let cleared: UpdateTeamRequest = serde_json::from_str(r#"{"color":null}"#).expect("parse");
        assert_eq!(cleared.color, Patch::Null);
        assert_eq!(
            serde_json::to_string(&cleared).expect("serialize"),
            r#"{"color":null}"#
        );
    }

    #[test]
    fn requests_refuse_unknown_fields() {
        assert!(serde_json::from_str::<UpdateTeamRequest>(r##"{"colour":"#112233"}"##).is_err());
        assert!(serde_json::from_str::<AddTeamMembersRequest>(
            r#"{"members":[{"user_id":"u","rank":1}]}"#
        )
        .is_err());
    }

    #[test]
    fn roles_travel_lower_case() {
        let body = UpdateTeamMemberRequest {
            role: TeamRole::Lead,
        };
        assert_eq!(
            serde_json::to_string(&body).expect("serialize"),
            r#"{"role":"lead"}"#
        );
    }
}
