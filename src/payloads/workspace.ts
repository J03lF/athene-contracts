/**
 * Workspace wire types (`/workspaces/*`, `/invite/*`): workspaces, their
 * settings, members and invitations.
 *
 * Generated from the Rust contracts in
 * `rust/athene-contracts/src/workspace.rs` by ts-rs; do not add
 * hand-written wire shapes here. Below the re-exports: value lists and
 * defaults the client needs at run time, and client helper types.
 */
import type { Assert, Covers } from '../internal/covers';
import type { ComposerProperty } from '../generated/workspace/ComposerProperty';
import type { UpdateWorkspaceRequest } from '../generated/workspace/UpdateWorkspaceRequest';

export type { ComposerPropertiesSetting } from '../generated/workspace/ComposerPropertiesSetting';
export type { ComposerProperty } from '../generated/workspace/ComposerProperty';
export type { CreateInviteRequest } from '../generated/workspace/CreateInviteRequest';
export type { CreateWorkspaceRequest } from '../generated/workspace/CreateWorkspaceRequest';
export type { DefaultAssignee } from '../generated/workspace/DefaultAssignee';
export type { InvitationResponse } from '../generated/workspace/InvitationResponse';
export type { InvitationStatus } from '../generated/workspace/InvitationStatus';
export type { InviteDetailsResponse } from '../generated/workspace/InviteDetailsResponse';
export type { JoinInviteResponse } from '../generated/workspace/JoinInviteResponse';
export type { UpdateMemberRoleRequest } from '../generated/workspace/UpdateMemberRoleRequest';
export type { UpdateWorkspaceRequest } from '../generated/workspace/UpdateWorkspaceRequest';
export type { WeekStart } from '../generated/workspace/WeekStart';
export type { WorkspaceMemberResponse } from '../generated/workspace/WorkspaceMemberResponse';
export type { WorkspaceMemberRole } from '../generated/workspace/WorkspaceMemberRole';
export type { WorkspaceResponse } from '../generated/workspace/WorkspaceResponse';
export type { WorkspaceSettings } from '../generated/workspace/WorkspaceSettings';
export type { WorkspaceSettingsPatch } from '../generated/workspace/WorkspaceSettingsPatch';
export {
  INVITE_DEFAULT_DAYS,
  INVITE_MAX_DAYS,
  LINK_INVITE_MAX_ROLE,
} from '../generated/workspace/invite-constants';

/** The composer properties a workspace may order and hide, in default order. */
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
] as const satisfies readonly ComposerProperty[];
type _ComposerPropertiesComplete = Assert<Covers<ComposerProperty, typeof COMPOSER_PROPERTIES>>;

/** The working day a workspace has when it has not set one. */
export const DEFAULT_WORK_DAY_START = '08:00';
export const DEFAULT_WORK_DAY_END = '18:00';
export const DEFAULT_WORK_DAYS: readonly number[] = [1, 2, 3, 4, 5];

/** A rename: the `PATCH /workspaces/:id` body with only the name. */
export type RenameWorkspaceRequest = Required<Pick<UpdateWorkspaceRequest, 'name'>>;
