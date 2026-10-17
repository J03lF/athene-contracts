export interface WorkspaceResponse {
  id: string;
  name: string;
  slug: string;
  logo_url?: string | null;
  owner_id: string;
  role: string;
  joined_at: string;
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

export interface CreateWorkspaceRequest {
  name: string;
}

/** Role a member holds in a workspace. */
export type WorkspaceMemberRole = 'owner' | 'admin' | 'member' | 'guest';

export interface WorkspaceMemberResponse {
  user_id: string;
  role: WorkspaceMemberRole;
  /** RFC3339 timestamp. */
  joined_at: string;
  /**
   * `null` when the membership outlived the user record. The row is still
   * returned so an orphaned membership stays visible instead of vanishing.
   */
  display_name: string | null;
  email: string | null;
}
