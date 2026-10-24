/**
 * Dashboard wire types (`/dashboard/*`), generated from
 * `rust/athene-contracts/src/dashboard.rs` by ts-rs. These routes answer
 * without the `{ data }` envelope.
 *
 * `FocusTicket.sequence` is the ticket's per-project number; the key
 * ("ATH-3") is composed on the client from it and the project's prefix.
 * `OnTrackResponse` is a state plus a number, never a sentence: the
 * sentence is written where the reader's language is known.
 */
export type { DashboardOverviewResponse } from '../generated/dashboard/DashboardOverviewResponse';
export type { FocusTab } from '../generated/dashboard/FocusTab';
export type { FocusTicket } from '../generated/dashboard/FocusTicket';
export type { KpiTile } from '../generated/dashboard/KpiTile';
export type { KpisResponse } from '../generated/dashboard/KpisResponse';
export type { OnTrackResponse } from '../generated/dashboard/OnTrackResponse';
export type { OnTrackState } from '../generated/dashboard/OnTrackState';
