/**
 * Rules on a workflow status (W4-G): the approval step (who may move a
 * ticket out of a status, four-eyes, allowed targets) and WIP limits (how
 * many tickets a status takes, per column or per person; warn or block).
 * Both per status of a workspace, overridable per project, off until set.
 *
 * Every member reads them (`GET /work/workspaces/:id/statuses/rules`, the
 * counts `GET /work/workspaces/:id/wip?project_id=`); owners and admins
 * set them (`PUT /work/workspaces/:id/statuses/:status_id/rules`).
 *
 * Generated from `rust/athene-contracts/src/status_rules.rs` by ts-rs; the
 * constants below mirror the ones there.
 */
export type { ApprovalMovers } from '../generated/workflow/ApprovalMovers';
export type { ApprovalNote } from '../generated/workflow/ApprovalNote';
export type { FourEyes } from '../generated/workflow/FourEyes';
export type { RulePlace } from '../generated/workflow/RulePlace';
export type { SetStatusRulesRequest } from '../generated/workflow/SetStatusRulesRequest';
export type { SetStatusRulesResponse } from '../generated/workflow/SetStatusRulesResponse';
export type { StatusApproval } from '../generated/workflow/StatusApproval';
export type { StatusRuleResponse } from '../generated/workflow/StatusRuleResponse';
export type { StatusWipCount } from '../generated/workflow/StatusWipCount';
export type { StatusWipLimit } from '../generated/workflow/StatusWipLimit';
export type { TicketWarning } from '../generated/workflow/TicketWarning';
export type { TicketWarningKind } from '../generated/workflow/TicketWarningKind';
export type { TicketWriteResponse } from '../generated/workflow/TicketWriteResponse';
export type { WipCountsQuery } from '../generated/workflow/WipCountsQuery';
export type { WipCountsResponse } from '../generated/workflow/WipCountsResponse';
export type { WipMode } from '../generated/workflow/WipMode';
export type { WipOverride } from '../generated/workflow/WipOverride';
export type { WipOverrideNote } from '../generated/workflow/WipOverrideNote';
export type { WipPersonCount } from '../generated/workflow/WipPersonCount';
export type { WipScope } from '../generated/workflow/WipScope';
export type { WorkflowRulesResponse } from '../generated/workflow/WorkflowRulesResponse';

/** `409` reasons (`details.reason`) of a status write an approval refuses. */
export const APPROVAL_NOT_ALLOWED_REASON = 'approval_not_allowed';
export const APPROVAL_FOUR_EYES_REASON = 'approval_four_eyes';
export const APPROVAL_TARGET_NOT_ALLOWED_REASON = 'approval_target_not_allowed';
/** `409` reason of a status write a blocking WIP limit refuses. */
export const WIP_LIMIT_REACHED_REASON = 'wip_limit_reached';

/** The highest WIP limit. */
export const WIP_LIMIT_MAX = 999;
/** The longest reason of a WIP override, in characters. */
export const WIP_OVERRIDE_REASON_MAX_CHARS = 500;
/** The most roles and teams an approval names (together). */
export const APPROVAL_MAX_MOVERS = 50;
