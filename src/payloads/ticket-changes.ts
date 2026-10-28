/**
 * Which tickets of a workspace changed (B5, W7-P). The core answers
 * `GET /work/workspaces/{id}/ticket-changes?since=`; the gateway streams
 * the same as server-sent events at `ticketStreamPath(workspaceId)`
 * (open with a stream ticket). A change names the ticket and its version,
 * never its content; one may come twice. Generated from
 * `rust/athene-contracts/src/ticket_changes.rs`.
 */
export type { TicketChange } from '../generated/work/TicketChange';
export type { TicketChangedEvent } from '../generated/work/TicketChangedEvent';
export type { TicketChangesQuery } from '../generated/work/TicketChangesQuery';
export type { TicketChangesResponse } from '../generated/work/TicketChangesResponse';
export type { TicketStreamCloseReason } from '../generated/work/TicketStreamCloseReason';
export type { TicketStreamClosed } from '../generated/work/TicketStreamClosed';

/** SSE event names of the gateway's ticket stream. */
export const TICKET_STREAM_EVENTS = {
  /** Once, when the watch has its starting point. */
  ready: 'ready',
  /** A `TicketChangedEvent`: tickets changed. */
  changed: 'ticket_changed',
  /** Changes may have been missed: read the sets again. */
  resync: 'resync',
  /** A `TicketStreamClosed` right before the stream ends on its own. */
  closed: 'closed',
} as const;

/** The gateway path (below `/api/v1`) of a workspace's ticket stream. */
export function ticketStreamPath(workspaceId: string): string {
  return `/work/workspaces/${encodeURIComponent(workspaceId)}/tickets/stream`;
}
