/**
 * Policy wire types (3.8a): the actions the server authorises, the
 * standing model and the capabilities answer
 * (`GET /work/workspaces/:workspace_id/capabilities`).
 *
 * Generated from `rust/athene-contracts/src/policy.rs` by ts-rs. The rules
 * themselves live there once (`allows`); their verdicts for a fixed set of
 * subjects are rendered to `src/generated/policy/policy-matrix.json`, which
 * the web tests its local rules against.
 *
 * `can(action, ctx) = actions[action] || (on_own[action] && isOwn(ctx))`.
 * A stranger gets 403, as for a workspace that does not exist.
 */
export type { Action } from '../generated/policy/Action';
export type { ActionKind } from '../generated/policy/ActionKind';
export type { ActionScope } from '../generated/policy/ActionScope';
export type { CapabilitiesResponse } from '../generated/policy/CapabilitiesResponse';
export type { Fact } from '../generated/policy/Fact';
export type { Standing } from '../generated/policy/Standing';
