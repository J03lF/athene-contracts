/**
 * Paged ticket lists (step 3.9): a page, its cursor vocabulary, the summary
 * projection and a ticket's effective due date. Generated from
 * `rust/athenos-contracts/src/listing.rs`; see `WorkApiClient.listTicketPage`.
 */
export type { CustomFieldFilter } from '../generated/work/CustomFieldFilter';
export type { CustomFieldOp } from '../generated/work/CustomFieldOp';
export type { DueSource } from '../generated/work/DueSource';
export type { EffectiveDue } from '../generated/work/EffectiveDue';
export type { Page } from '../generated/work/Page';
export type { PageMeta } from '../generated/work/PageMeta';
export type { SortOrder } from '../generated/work/SortOrder';
export type { TicketFacet } from '../generated/work/TicketFacet';
export type { TicketFacets } from '../generated/work/TicketFacets';
export type { TicketFields } from '../generated/work/TicketFields';
export type { TicketSort } from '../generated/work/TicketSort';
export type { TicketSummary } from '../generated/work/TicketSummary';
export type { WorkspaceSprintsQuery } from '../generated/work/WorkspaceSprintsQuery';

/**
 * List filters and facets of W7-P on `ListTicketsQuery`: `reporter` (`me`
 * or ids), `ids` (at most `TICKET_LIST_MAX_IDS`), `own_due` (the date
 * filters read the ticket's own due date, not its sprint's) and `facets`
 * (comma-separated `TicketFacet`s; `meta.facets` answers the counts, each
 * facet ignoring its own filter).
 */
export const TICKET_FACETS = ['status', 'category', 'type', 'assignee', 'priority'] as const;
