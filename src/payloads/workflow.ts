/**
 * A workspace's own ticket statuses (W4-F, "Workflow"). Every workspace has
 * the five built-in statuses (keys unchanged) and the ones it adds; a
 * ticket's `status` is a status key, and what it means is its
 * `status_category` (`open`, `in_progress`, `blocked`, `done`). A board
 * column is a status; a queue waits in any status that is not a `done` one.
 *
 * Every member reads the statuses; owners and admins edit them
 * (`workspace.settings.edit`). Per project a status can be hidden: not
 * offered there, but tickets already in it still show.
 *
 * Generated from `rust/athene-contracts/src/workflow.rs` by ts-rs; the
 * constants below mirror the ones there.
 */
import type { StatusCategory } from '../generated/workflow/StatusCategory';

export type { CreateWorkflowStatusRequest } from '../generated/workflow/CreateWorkflowStatusRequest';
export type { DeleteWorkflowStatusRequest } from '../generated/workflow/DeleteWorkflowStatusRequest';
export type { DeleteWorkflowStatusResponse } from '../generated/workflow/DeleteWorkflowStatusResponse';
export type { ProjectHiddenStatusesResponse } from '../generated/workflow/ProjectHiddenStatusesResponse';
export type { ReorderWorkflowStatusesRequest } from '../generated/workflow/ReorderWorkflowStatusesRequest';
export type { SetHiddenStatusesRequest } from '../generated/workflow/SetHiddenStatusesRequest';
export type { StatusCategory } from '../generated/workflow/StatusCategory';
export type { UpdateWorkflowStatusRequest } from '../generated/workflow/UpdateWorkflowStatusRequest';
export type { WorkflowQuery } from '../generated/workflow/WorkflowQuery';
export type { WorkflowStatusActivity } from '../generated/workflow/WorkflowStatusActivity';
export type { WorkflowStatusExpected } from '../generated/workflow/WorkflowStatusExpected';
export type { WorkflowStatusResponse } from '../generated/workflow/WorkflowStatusResponse';
export type { WorkspaceWorkflowResponse } from '../generated/workflow/WorkspaceWorkflowResponse';

/** Every category, in board order. */
export const STATUS_CATEGORIES: readonly StatusCategory[] = ['open', 'in_progress', 'blocked', 'done'];

/** The built-in statuses: key, category, default order and colour token. */
export const BUILTIN_STATUSES: readonly {
  readonly key: string;
  readonly category: StatusCategory;
  readonly position: number;
  readonly color: string;
}[] = [
  { key: 'open', category: 'open', position: 0, color: 'neutral' },
  { key: 'in_progress', category: 'in_progress', position: 1, color: 'blue' },
  { key: 'in_review', category: 'in_progress', position: 2, color: 'purple' },
  { key: 'blocked', category: 'blocked', position: 3, color: 'red' },
  { key: 'done', category: 'done', position: 4, color: 'green' },
];

/** The status a new ticket starts in; it cannot be hidden. */
export const START_STATUS = 'open';
/** The status a new queue suggests ("Bereit zum Test"). */
export const DEFAULT_QUEUE_STATUS = 'in_review';
/** Where "Zurückschicken" sends a ticket. */
export const HAND_BACK_STATUS = 'in_progress';
/** Older values a ticket may still hold; they count as `done`. */
export const LEGACY_DONE_KEYS: readonly string[] = ['closed', 'resolved'];

/** The longest status key: `^[a-z][a-z0-9_]{0,31}$`. */
export const STATUS_KEY_MAX_CHARS = 32;
/** The longest status name, in characters (one line). */
export const STATUS_NAME_MAX_CHARS = 40;
/** The most statuses a workspace holds, archived ones included. */
export const WORKFLOW_MAX_STATUSES = 40;
/** The colour tokens a status may use (the ticket types' tokens). */
export const STATUS_COLORS: readonly string[] = ['neutral', 'blue', 'green', 'amber', 'red', 'purple', 'pink'];

/** `409` reasons (`details.reason`) of a status edit or delete. */
export const STATUS_IN_USE_REASON = 'status_in_use';
export const STATUS_HAS_QUEUES_REASON = 'status_has_queues';
/** `400` reasons (`details.reason`) of a ticket write into a status. */
export const STATUS_UNKNOWN_REASON = 'status_unknown';
export const STATUS_ARCHIVED_REASON = 'status_archived';
export const STATUS_HIDDEN_REASON = 'status_hidden';
/** `400` reason of a queue on a `done`, archived or unknown status. */
export const QUEUE_STATUS_DONE_REASON = 'queue_status_done';

/** The category a key has by itself (built-in or legacy), else `null`. */
export function fixedStatusCategory(key: string): StatusCategory | null {
  const builtin = BUILTIN_STATUSES.find((s) => s.key === key);
  if (builtin) {
    return builtin.category;
  }
  return LEGACY_DONE_KEYS.includes(key) ? 'done' : null;
}
