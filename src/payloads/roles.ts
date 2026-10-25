/**
 * Workspace roles (custom roles, phase A): named bundles of permissions
 * per workspace (`/workspaces/:workspace_id/roles`). Every member holds
 * exactly one; Owner and Admin are fixed, Mitglied and Gast can be edited
 * but not deleted, a guest role holds at most `GUEST_CEILING`.
 *
 * Generated from `rust/athene-contracts/src/roles.rs` by ts-rs.
 */
export type { AnswerRolesNoticeRequest } from '../generated/roles/AnswerRolesNoticeRequest';
export type { CreatePresetRolesRequest } from '../generated/roles/CreatePresetRolesRequest';
export type { CreateRoleRequest } from '../generated/roles/CreateRoleRequest';
export type { DeleteRoleRequest } from '../generated/roles/DeleteRoleRequest';
export type { DeleteRoleResponse } from '../generated/roles/DeleteRoleResponse';
export type { EffectiveRoleResponse } from '../generated/roles/EffectiveRoleResponse';
export type { ReorderRolesRequest } from '../generated/roles/ReorderRolesRequest';
export type { RolePresetResponse } from '../generated/roles/RolePresetResponse';
export type { RolesNotice } from '../generated/roles/RolesNotice';
export type { UpdateRoleRequest } from '../generated/roles/UpdateRoleRequest';
export type { WorkspaceRoleResponse } from '../generated/roles/WorkspaceRoleResponse';
export type { WorkspaceRolesResponse } from '../generated/roles/WorkspaceRolesResponse';
