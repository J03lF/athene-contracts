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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct InviteDetailsResponse {
    pub workspace_name: String,
    pub workspace_slug: String,
    pub role: String,
    pub email_restricted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct JoinInviteResponse {
    pub workspace_id: String,
    pub workspace_slug: String,
    pub already_member: bool,
}

pub type WorkspaceListResponse = crate::ListResponse<WorkspaceResponse>;
