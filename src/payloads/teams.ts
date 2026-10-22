/**
 * Teams wire shapes — named groups of a workspace's members. Mirrored by
 * hand in `rust/athene-contracts/src/teams.rs`.
 *
 * Every response is wrapped as `{ data: … }`. Ids are UUID strings,
 * timestamps RFC 3339.
 *
 * Server rules: name 1..80 chars on one line, unique per workspace ignoring
 * case; key 2..10 of `A-Z`/`0-9` starting with a letter, stored upper-case,
 * unique per workspace — suggested from the name (numbered on a clash) when
 * absent; description ≤ 500 chars; colour `#RRGGBB`. Only members of the
 * workspace can be in its teams, and leaving the workspace leaves its teams.
 *
 * Permissions: workspace owners and admins create, edit and delete teams
 * and add, remove and re-role their members; every workspace member reads;
 * anyone may remove themselves from a team (leave). A non-member, or an id
 * of another workspace, answers 403. A name or key clash answers 409.
 */

export const TEAM_MAX_NAME_CHARS = 80;
export const TEAM_MAX_DESCRIPTION_CHARS = 500;
export const TEAM_MIN_KEY_CHARS = 2;
export const TEAM_MAX_KEY_CHARS = 10;
/** How many members a team in a list carries along with `member_count`. */
export const TEAM_MEMBER_PREVIEW = 5;
/** The most people one add-members request may name. */
export const TEAM_MAX_MEMBERS_PER_REQUEST = 100;

export type TeamRole = 'lead' | 'member';

export interface TeamMemberResponse {
  user_id: string;
  role: TeamRole;
  added_at: string;
  /** `null` when the user record is gone. */
  display_name: string | null;
  email: string | null;
}

export interface TeamResponse {
  id: string;
  workspace_id: string;
  name: string;
  key: string;
  description: string | null;
  /** `#RRGGBB`. */
  color: string | null;
  /** Everyone in the team. */
  member_count: number;
  /**
   * Leads first, then by when they joined. In a list, the first
   * `TEAM_MEMBER_PREVIEW`; for a single team, everyone.
   */
  members: TeamMemberResponse[];
  /** The caller's role in the team; `null` when they are not in it. */
  my_role: TeamRole | null;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface TeamMemberInput {
  user_id: string;
  /** Defaults to `member`. */
  role?: TeamRole;
}

export interface CreateTeamRequest {
  /** 1..80 chars. */
  name: string;
  /** Absent = suggested from the name. */
  key?: string;
  /** ≤ 500 chars. */
  description?: string | null;
  /** `#RRGGBB`. */
  color?: string | null;
  /** People to put in the team straight away; each a workspace member. */
  members?: TeamMemberInput[];
}

/** An absent key leaves that part as it is; `null` clears it. */
export interface UpdateTeamRequest {
  name?: string;
  key?: string;
  description?: string | null;
  color?: string | null;
}

/**
 * Either `members` (a role each) or `user_ids` sharing `role`, or both.
 * Someone already in the team keeps their role.
 */
export interface AddTeamMembersRequest {
  members?: TeamMemberInput[];
  user_ids?: string[];
  /** For `user_ids`; defaults to `member`. */
  role?: TeamRole;
}

export interface UpdateTeamMemberRequest {
  role: TeamRole;
}

export interface TeamListQuery {
  /** A user id, or `'me'`: only the teams that person is in. */
  member?: string;
}
