/**
 * Workspace roles (custom roles, phase A): named bundles of permissions
 * per workspace (`/workspaces/:workspace_id/roles`). Every member has one
 * base level (the tier) and holds any number of roles besides it (W4-G):
 * their rights are the union. Owner and Admin are fixed, Mitglied and Gast
 * can be edited but not deleted, a guest role holds at most
 * `GUEST_CEILING`.
 *
 * Generated from `rust/athenos-contracts/src/roles.rs` by ts-rs.
 */
export type { AddMemberRoleRequest } from '../generated/roles/AddMemberRoleRequest';
export type { AnswerRolesNoticeRequest } from '../generated/roles/AnswerRolesNoticeRequest';
export type { CreatePresetRolesRequest } from '../generated/roles/CreatePresetRolesRequest';
export type { CreateRoleRequest } from '../generated/roles/CreateRoleRequest';
export type { DeleteRoleRequest } from '../generated/roles/DeleteRoleRequest';
export type { DeleteRoleResponse } from '../generated/roles/DeleteRoleResponse';
export type { EffectiveRoleResponse } from '../generated/roles/EffectiveRoleResponse';
export type { MemberRoleName } from '../generated/roles/MemberRoleName';
export type { MemberRoleRef } from '../generated/roles/MemberRoleRef';
export type { MemberRolesActivity } from '../generated/roles/MemberRolesActivity';
export type { MemberRolesResponse } from '../generated/roles/MemberRolesResponse';
export type { ReorderRolesRequest } from '../generated/roles/ReorderRolesRequest';
export type { RolePresetResponse } from '../generated/roles/RolePresetResponse';
export type { RolesNotice } from '../generated/roles/RolesNotice';
export type { SetMemberRolesRequest } from '../generated/roles/SetMemberRolesRequest';
export type { UpdateRoleRequest } from '../generated/roles/UpdateRoleRequest';
export type { WorkspaceRoleResponse } from '../generated/roles/WorkspaceRoleResponse';
export type { WorkspaceRolesResponse } from '../generated/roles/WorkspaceRolesResponse';

/** The most roles one member holds besides their tier (W4-G). */
export const MEMBER_MAX_ROLES = 50;
