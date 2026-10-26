/**
 * Activity feed wire types, generated from
 * `rust/athene-contracts/src/activity.rs` by ts-rs.
 */
import type { ActivityAction } from '../generated/activity/ActivityAction';
import { ACTIVITY_ACTIONS } from '../generated/activity/activity-actions';

export type { ActivityEventResponse } from '../generated/activity/ActivityEventResponse';

/**
 * Every verb the server records: a closed set, generated from the Rust enum
 * (BE-17, step 3.10). A new verb is a new variant there; the web's exhaustive
 * `switch` then fails to compile until it narrates it.
 *
 * On the wire (`ActivityEventResponse.action`) the verb stays a string: the
 * feed is history, and an entry from an older or newer server must still
 * render, as a generic line. {@link parseActivityAction} is the one place
 * that turns the wire string into this type.
 */
export type { ActivityAction } from '../generated/activity/ActivityAction';
export { ACTIVITY_ACTIONS } from '../generated/activity/activity-actions';

const KNOWN: ReadonlySet<string> = new Set(ACTIVITY_ACTIONS);

/** The verb `action` names, or `null` for one this build does not know. */
export function parseActivityAction(action: string): ActivityAction | null {
  return KNOWN.has(action) ? (action as ActivityAction) : null;
}
