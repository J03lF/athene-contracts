export interface WorkspaceResponse {
  id: string;
  name: string;
  slug: string;
  logo_url?: string | null;
  owner_id: string;
  created_at: string;
}

export interface InviteDetailsResponse {
  workspace_name: string;
  workspace_slug: string;
  role: string;
  email_restricted: boolean;
}

export interface JoinInviteResponse {
  workspace_id: string;
  workspace_slug: string;
  already_member: boolean;
}
