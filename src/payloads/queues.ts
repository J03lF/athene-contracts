/**
 * Queues and hand-offs (custom roles, phase C1): a queue maps a ticket
 * status to a target role. Entering the status hands the ticket off
 * (unassigned, `handoff_from` noted); a member of the role takes it
 * atomically (`409 ATH-RES-005` when someone was faster) or hands it back
 * to the person it came from, with a comment.
 *
 * Generated from `rust/athenos-contracts/src/queues.rs` by ts-rs.
 */
export type { CreateQueueRequest } from '../generated/queues/CreateQueueRequest';
export type { DeleteQueueResponse } from '../generated/queues/DeleteQueueResponse';
export type { HandBackTicketRequest } from '../generated/queues/HandBackTicketRequest';
export type { MyQueuesResponse } from '../generated/queues/MyQueuesResponse';
export type { QueueActivity } from '../generated/queues/QueueActivity';
export type { QueueExpected } from '../generated/queues/QueueExpected';
export type { QueueResponse } from '../generated/queues/QueueResponse';
export type { QueueTargetKind } from '../generated/queues/QueueTargetKind';
export type { QueueTickets } from '../generated/queues/QueueTickets';
export type { TakeTicketRequest } from '../generated/queues/TakeTicketRequest';
export type { TakenBy } from '../generated/queues/TakenBy';
export type { UpdateQueueRequest } from '../generated/queues/UpdateQueueRequest';
export type { WorkspaceQueuesResponse } from '../generated/queues/WorkspaceQueuesResponse';

/** `409`: somebody else took the ticket first (`details.taken_by`). */
export const ALREADY_TAKEN_CODE = 'ATH-RES-005';
/** `409`: the ticket waits in no queue (any more). */
export const NOT_IN_QUEUE_CODE = 'ATH-RES-006';
/** The longest queue name, in characters. */
export const QUEUE_MAX_NAME_CHARS = 60;
/** The longest status display name, in characters. */
export const STATUS_LABEL_MAX_CHARS = 40;
