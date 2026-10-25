/**
 * Policy wire types (3.8a, custom roles phase A): the actions the server
 * authorises, the permissions roles are made of, the standing model and
 * the capabilities answer (`GET /work/workspaces/:workspace_id/capabilities`).
 *
 * Generated from `rust/athene-contracts/src/policy.rs` by ts-rs. The rules
 * themselves live there once (`allows`); their verdicts for a fixed set of
 * subjects are rendered to `src/generated/policy/policy-matrix.json`, which
 * the web tests its loading fallback against.
 *
 * `can(action, ctx) = actions[action] || (on_own[action] && isOwn(ctx))`.
 * A stranger gets 403, as for a workspace that does not exist.
 */
export type { Action } from '../generated/policy/Action';
export type { ActionKind } from '../generated/policy/ActionKind';
export type { ActionScope } from '../generated/policy/ActionScope';
export type { CapabilitiesResponse } from '../generated/policy/CapabilitiesResponse';
export type { CapabilitiesRole } from '../generated/policy/CapabilitiesRole';
export type { EffectivePermission } from '../generated/policy/EffectivePermission';
export type { Permission } from '../generated/policy/Permission';
export type { PermissionGroup } from '../generated/policy/PermissionGroup';
export type { PermissionSource } from '../generated/policy/PermissionSource';
export type { RolePreset } from '../generated/policy/RolePreset';
export {
  GUEST_CEILING,
  MEMBER_DEFAULT,
  PERMISSIONS,
  PLATFORM_MEMBER_EXTRA,
  POLICY_REVISION,
  ROLE_MAX_DESCRIPTION_CHARS,
  ROLE_MAX_NAME_CHARS,
  ROLE_MAX_PER_WORKSPACE,
  ROLE_PRESETS,
} from '../generated/policy/permission-catalog';
/** The generated matrix (for specs; tree-shaken out of an app that does not import it). */
export {
  POLICY_MATRIX,
  type PolicyMatrix,
  type PolicyMatrixFacts,
} from '../generated/policy/policy-matrix';
export type { Fact } from '../generated/policy/Fact';
export type { Standing } from '../generated/policy/Standing';
