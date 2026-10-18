// Dashboard aggregator payloads (synced with rust crate `dashboard.rs`).

import type { ActivityEventResponse } from './activity';

export interface KpiTile {
  label: string;
  value: number;
  delta?: number;
}

export interface KpisResponse {
  assigned: KpiTile;
  due_this_week: KpiTile;
  in_review: KpiTile;
  closed_this_week: KpiTile;
}

export interface FocusTicket {
  id: string;
  /**
   * The ticket's per-project number. The key — "ATH-3" — is this plus the
   * project's prefix, composed on the client, which holds every project
   * and one definition of what a ticket key looks like.
   */
  sequence: number;
  title: string;
  status: string;
  priority: string;
  assignee_id: string | null;
  project_id: string;
  due_date: string | null;
}

/**
 * How the week is going, as a state plus a number.
 *
 * Not a sentence: the server used to compose "You have 2 overdue items"
 * and the client printed it, which is one language on every build. The
 * three states and the count are everything the sentence carried, so
 * the sentence is written where the reader's language is known.
 */
export type OnTrackState =
  /** At least one overdue item in the focus list. `count` is how many. */
  | 'overdue'
  /** A heavy week — `count` items due. */
  | 'busy_week'
  /** Nothing pressing. `count` is 0. */
  | 'on_track';

export interface OnTrackResponse {
  state: OnTrackState;
  count: number;
}

export interface DashboardOverviewResponse {
  kpis: KpisResponse;
  focus: FocusTicket[];
  recent_activity: ActivityEventResponse[];
  on_track: OnTrackResponse;
}

export type FocusTab = 'today' | 'assigned' | 'review';
