/**
 * Activity feed wire types, generated from
 * `rust/athene-contracts/src/activity.rs` by ts-rs.
 */
export type { ActivityEventResponse } from '../generated/activity/ActivityEventResponse';

/**
 * The verbs the ticket feed emits today. Kept as a union of known values
 * widened with `string`, because the feed is an open vocabulary: a module
 * added later may emit a verb this build has never heard of, and the UI must
 * render that as a generic entry rather than crash or drop it. On the wire
 * (`ActivityEventResponse.action`) it is a plain string.
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
