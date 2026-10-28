/**
 * Ticket and doc search wire types (3.9): `GET /work/workspaces/:workspace_id/search`.
 *
 * Generated from `rust/athenos-contracts/src/search.rs` by ts-rs. Highlights
 * are `TextPart`s, never markup; secrets in tickets come back masked as `***`.
 */
export type { DocSearchHit } from '../generated/search/DocSearchHit';
export type { FacetCount } from '../generated/search/FacetCount';
export type { SearchHitKind } from '../generated/search/SearchHitKind';
export type { SearchTicketsQuery } from '../generated/search/SearchTicketsQuery';
export type { TextPart } from '../generated/search/TextPart';
export type { TicketSearchFacets } from '../generated/search/TicketSearchFacets';
export type { TicketSearchHit } from '../generated/search/TicketSearchHit';
export type { TicketSearchResponse } from '../generated/search/TicketSearchResponse';
