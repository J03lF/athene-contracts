import type { TicketTypeColor } from './ticket-types';

export interface WorkspaceResponse {
  id: string;
  name: string;
  slug: string;
  logo_url?: string | null;
  owner_id: string;
  role: string;
  joined_at: string;
  created_at: string;
  /** ≤ 500 chars. */
  description: string | null;
  /** A `TicketTypeColor` token (`blue`, `purple`, …), never hex. */
  color: TicketTypeColor | null;
  /** Every key present; `null` = the product default. */
  settings: WorkspaceSettings;
}

/** The composer properties a workspace may order and hide. */
export const COMPOSER_PROPERTIES = [
  'type',
  'project',
  'status',
  'priority',
  'assignee',
  'due',
  'labels',
  'points',
  'sprint',
  'release',
] as const;
export type ComposerProperty = (typeof COMPOSER_PROPERTIES)[number];

export interface ComposerPropertiesSetting {
  /** Display order; properties not listed follow in the default order. */
  order: ComposerProperty[];
  hidden: ComposerProperty[];
}

export interface WorkspaceSettings {
  /** A project of this workspace (400 otherwise). */
  default_project_id: string | null;
  default_assignee: 'creator' | 'none' | null;
  default_priority: 'low' | 'medium' | 'high' | 'critical' | null;
  /** When `true`, `POST /work/tickets` without `story_points` answers 400. */
  require_estimate: boolean | null;
  composer_properties: ComposerPropertiesSetting | null;
  sprint_week_start: 'monday' | 'sunday' | null;
  /** `"HH:MM"` (24h), when the working day starts; `null` = {@link DEFAULT_WORK_DAY_START}. */
  work_day_start: string | null;
  /** `"HH:MM"`, after `work_day_start` (400 otherwise); `null` = {@link DEFAULT_WORK_DAY_END}. */
  work_day_end: string | null;
  /** ISO weekdays 1 (Mon) – 7 (Sun), ascending, non-empty; `null` = {@link DEFAULT_WORK_DAYS}. */
  work_days: number[] | null;
}

/** The working day a workspace has when it has not set one. */
export const DEFAULT_WORK_DAY_START = '08:00';
export const DEFAULT_WORK_DAY_END = '18:00';
export const DEFAULT_WORK_DAYS: readonly number[] = [1, 2, 3, 4, 5];

/**
 * `PATCH /workspaces/:id` — owners and admins (403 otherwise). An absent key
 * leaves that part as it is; `null` clears `description` / `color`.
 * `settings` is merged key by key: a value sets the key, `null` unsets it,
 * an unknown key or a bad value refuses the whole patch (400). An empty
 * patch is 400.
 */
export interface UpdateWorkspaceRequest {
  name?: string;
  description?: string | null;
  color?: TicketTypeColor | null;
  settings?: Partial<{ [K in keyof WorkspaceSettings]: WorkspaceSettings[K] | null }>;
}

export interface InviteDetailsResponse {
  workspace_name: string;
  workspace_slug: string;
  role: string;
  email_restricted: boolean;
  /** Display name of whoever created the invite; `null` when that user is gone. */
  invited_by_name: string | null;
  /**
   * The address the invite is restricted to, in full; `null` for an open
   * link. Joining is refused to any other account.
   */
  invited_email: string | null;
  /**
   * The signed-in caller is already a member. Then a spent, revoked or
   * expired invite is answered too (instead of 400), so the page can say
   * "you're already in <workspace_name>".
   */
  already_member: boolean;
  /** Only when `already_member`. */
  workspace_id: string | null;
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

/**
 * `PATCH /workspaces/:id/members/:user_id`. Owners and admins only; only an
 * owner may grant, change or remove the owner role; the last owner cannot
 * be demoted (409).
 */
export interface UpdateMemberRoleRequest {
  role: WorkspaceMemberRole;
}

export interface RenameWorkspaceRequest {
  name: string;
}
