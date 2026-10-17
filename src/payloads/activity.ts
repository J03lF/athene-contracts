// Activity feed payloads (synced with rust crate `activity.rs`).

/**
 * The verbs the ticket feed emits today. Kept as a union of known values
 * widened with `string`, because the feed is an open vocabulary: a module
 * added later may emit a verb this build has never heard of, and the UI must
 * render that as a generic entry rather than crash or drop it.
 */
export type ActivityAction =
  | 'ticket_created'
  | 'ticket_status_changed'
  | 'ticket_assigned'
  | 'ticket_commented'
  | 'ticket_updated'
  | 'ticket_custom_values_updated'
  | 'ticket_linked'
  | (string & {});

export interface ActivityEventResponse {
  id: string;
  workspace_id: string;
  /** `null` for system or automation events with no human actor. */
  actor_id: string | null;
  action: ActivityAction;
  /** `ticket`, `project`, `sprint`, `milestone`, ... */
  target_type: string;
  /**
   * Short user-authored text (e.g. a ticket title). Machine-derived detail
   * is **not** here — it lives in `metadata`, so the client renders it in
   * the reader's language instead of printing a server-composed sentence.
   */
  detail: string | null;
  target_id: string;
  /**
   * Structured payload for the verb. `ticket_status_changed` carries
   * `{ from, to }` in the ticket-status vocabulary; `ticket_assigned` carries
   * `{ assignee_id }`; `ticket_updated` carries `{ fields: string[] }`;
   * `ticket_linked` carries `{ relation, to_ticket_id }`.
   */
  metadata: Record<string, unknown> | null;
  /** ISO-8601 (RFC 3339). */
  created_at: string;
}
