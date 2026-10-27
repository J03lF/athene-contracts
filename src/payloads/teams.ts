/**
 * Teams wire types: named groups of a workspace's members. Generated from
 * `rust/athene-contracts/src/teams.rs` by ts-rs; the limits below mirror
 * the constants there.
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

export type { AddTeamMembersRequest } from '../generated/teams/AddTeamMembersRequest';
export type { CreateTeamRequest } from '../generated/teams/CreateTeamRequest';
export type { DeleteTeamRequest } from '../generated/teams/DeleteTeamRequest';
export type { DeleteTeamResponse } from '../generated/teams/DeleteTeamResponse';
export type { TeamListQuery } from '../generated/teams/TeamListQuery';
export type { TeamMemberInput } from '../generated/teams/TeamMemberInput';
export type { TeamMemberResponse } from '../generated/teams/TeamMemberResponse';
export type { TeamResponse } from '../generated/teams/TeamResponse';
export type { TeamRole } from '../generated/teams/TeamRole';
export type { UpdateTeamMemberRequest } from '../generated/teams/UpdateTeamMemberRequest';
export type { UpdateTeamRequest } from '../generated/teams/UpdateTeamRequest';

/** `409` reason of a plain delete: queues target the team (`details.queues`). */
export const TEAM_HAS_QUEUES_REASON = 'team_has_queues';
