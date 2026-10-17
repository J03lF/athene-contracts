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

export type InvitationStatus = 'active' | 'used' | 'expired' | 'revoked';

/**
 * An invitation to a workspace.
 *
 * Carries the `token` because the point of creating one is a link to send.
 * Only members may read the list — every live token in it is a way in.
 */
export interface InvitationResponse {
  id: string;
  workspace_id: string;
  token: string;
  /** Addressed to one person, or `null` for a link anybody may use. */
  email: string | null;
  role: WorkspaceMemberRole;
  invited_by: string;
  status: InvitationStatus;
  /** Only meaningful for a link invite; `null` means unlimited. */
  max_uses: number | null;
  use_count: number;
  /** RFC3339, or `null` when it does not expire on its own. */
  expires_at: string | null;
  created_at: string;
}

export interface CreateInviteRequest {
  /** `member` when omitted. Never more than the inviter holds. */
  role?: WorkspaceMemberRole;
  /** Omit or send `null` for a link anybody may use. */
  email?: string | null;
  max_uses?: number | null;
  /** RFC3339. */
  expires_at?: string | null;
}
