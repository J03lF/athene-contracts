/**
 * Permission overrides (custom roles, phase B): at a place — a project, a
 * docs area, or the workspace — a role or a team says allow, deny or
 * inherit per permission (`/workspaces/:workspace_id/overrides`).
 *
 * Generated types from `rust/athene-contracts/src/overrides.rs` by ts-rs,
 * and the resolution itself as a TypeScript port of `overrides::resolve`
 * and `overrides::resolve_traced`, held to the same cases as the Rust one
 * (`OVERRIDE_CASES`, `src/generated/policy/override-cases.json`).
 *
 * 1. Owner and admin: everything; no override reaches them.
 * 2. The role's ticks are the base.
 * 3. Per level, general to specific (workspace, project, docs area): the
 *    role's override there, then the member's teams' (allow wins among
 *    teams). Inherit says nothing.
 * 4. A guest is cut to the guest ceiling after everything.
 */
import type { EffectivePermission } from '../generated/policy/EffectivePermission';
import type { OverrideLevel } from '../generated/policy/OverrideLevel';
import type { OverrideState } from '../generated/policy/OverrideState';
import type { Permission } from '../generated/policy/Permission';
import type { PermissionStep } from '../generated/policy/PermissionStep';
import type { WorkspaceMemberRole } from '../generated/workspace/WorkspaceMemberRole';
import { GUEST_CEILING, PERMISSIONS } from '../generated/policy/permission-catalog';

export type { OverrideLevel } from '../generated/policy/OverrideLevel';
export type { OverridePlace } from '../generated/policy/OverridePlace';
export type { OverrideState } from '../generated/policy/OverrideState';
export type { OverrideTarget } from '../generated/policy/OverrideTarget';
export type { PermissionStep } from '../generated/policy/PermissionStep';
export type { PlaceQuery } from '../generated/policy/PlaceQuery';
export type { TeamOverrideMasks } from '../generated/policy/TeamOverrideMasks';
export type { PermissionOverrideResponse } from '../generated/roles/PermissionOverrideResponse';
export type { SetOverrideRequest } from '../generated/roles/SetOverrideRequest';
export type { SetOverrideResponse } from '../generated/roles/SetOverrideResponse';
export type { WorkspaceOverridesResponse } from '../generated/roles/WorkspaceOverridesResponse';
export { OVERRIDABLE } from '../generated/policy/permission-catalog';
/** The shared cases (for specs; tree-shaken out of an app that does not import it). */
export { OVERRIDE_CASES, type OverrideCase } from '../generated/policy/override-cases';

/** One override's word on one permission. */
export function overrideState(
  allow: ReadonlyArray<Permission>,
  deny: ReadonlyArray<Permission>,
  permission: Permission,
): OverrideState {
  // A permission in both reads as denied (the safe reading of a bad row).
  if (deny.includes(permission)) return 'deny';
  if (allow.includes(permission)) return 'allow';
  return 'inherit';
}

function fixed(tier: WorkspaceMemberRole): boolean {
  return tier === 'owner' || tier === 'admin';
}

/** One permission through the levels, with its chain. */
export function resolvePermission(
  tier: WorkspaceMemberRole,
  base: ReadonlyArray<Permission>,
  levels: ReadonlyArray<OverrideLevel>,
  permission: Permission,
): EffectivePermission {
  if (fixed(tier)) {
    return {
      permission,
      allowed: true,
      source: 'fixed',
      chain: [{ source: 'fixed', allowed: true }],
    };
  }
  const chain: PermissionStep[] = [
    { source: 'role', allowed: base.includes(permission) },
  ];
  let decided = 0;
  for (const level of levels) {
    const role = overrideState(level.role_allow, level.role_deny, permission);
    if (role !== 'inherit') {
      chain.push({
        source: 'role_override',
        allowed: role === 'allow',
        place: level.place,
        place_id: level.place_id,
      });
      decided = chain.length - 1;
    }
    let allowing: number | null = null;
    let denying: number | null = null;
    for (const team of level.teams) {
      const state = overrideState(team.allow, team.deny, permission);
      if (state === 'inherit') continue;
      chain.push({
        source: 'team_override',
        allowed: state === 'allow',
        place: level.place,
        place_id: level.place_id,
        team_id: team.team_id,
      });
      const at = chain.length - 1;
      if (state === 'allow') allowing ??= at;
      else denying ??= at;
    }
    const team = allowing ?? denying;
    if (team !== null) decided = team;
  }
  const decider = chain[decided];
  if (tier === 'guest' && !GUEST_CEILING.includes(permission)) {
    chain.push({ source: 'guest_ceiling', allowed: false });
    return { permission, allowed: false, source: 'guest_ceiling', chain };
  }
  const out: EffectivePermission = {
    permission,
    allowed: decider.allowed,
    source: decider.source,
    chain,
  };
  if (decider.place !== undefined) out.place = decider.place;
  if (decider.place_id !== undefined) out.place_id = decider.place_id;
  if (decider.team_id !== undefined) out.team_id = decider.team_id;
  return out;
}

/** Every permission, in the editor's order (`overrides::resolve_traced`). */
export function resolveOverrides(
  tier: WorkspaceMemberRole,
  base: ReadonlyArray<Permission>,
  levels: ReadonlyArray<OverrideLevel>,
): EffectivePermission[] {
  return PERMISSIONS.map((p) => resolvePermission(tier, base, levels, p.permission));
}

/** One held role's path for [`resolveAnyPermission`]: its ticks and its levels. */
export interface HeldRolePath {
  base: ReadonlyArray<Permission>;
  levels: ReadonlyArray<OverrideLevel>;
}

/**
 * Several roles per member (W4-G, `overrides::resolve_any`): a permission
 * is allowed when any held role's path allows it — a role's deny at a
 * place never takes away what another held role is allowed there. A
 * member who holds no role resolves one path without ticks.
 */
export function resolveAnyPermission(
  tier: WorkspaceMemberRole,
  paths: ReadonlyArray<HeldRolePath>,
  permission: Permission,
): boolean {
  return paths.some((path) => resolvePermission(tier, path.base, path.levels, permission).allowed);
}
