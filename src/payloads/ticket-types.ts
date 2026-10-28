/**
 * Ticket-type wire types: the configurable kinds of ticket per workspace.
 * Generated from `rust/athenos-contracts/src/ticket_types.rs` by ts-rs; the
 * value lists below mirror the constants there.
 *
 * Every response is wrapped as `{ data: … }`. Ids are UUID strings,
 * timestamps RFC 3339.
 *
 * Server rules:
 * - Every workspace starts with `task` (default), `bug`, `feature`,
 *   `support` and `epic` (hierarchy level 1). Their keys are what
 *   `TicketResponse.ticket_type` carries, so old tickets stay valid.
 * - `key` (`^[a-z][a-z0-9_]{0,31}$`) is chosen at creation — derived from
 *   the name when absent — unique per workspace (archived types included)
 *   and immutable.
 * - `name` 1..40 chars, unique per workspace among live types (case
 *   ignored); `description` ≤ 200; `icon` from `TICKET_TYPE_ICONS`; `color`
 *   from `TICKET_TYPE_COLORS` (a token name, not hex).
 * - `description_template`: optional Markdown (≤ 8 KB, blank = none) the
 *   create composer prefills an empty description with. Never seeded.
 * - Exactly one live type is the default (used when a ticket is created
 *   without a type). The default cannot be archived; the last live type
 *   cannot be archived.
 * - A type in use is archived, not deleted: archived types keep their
 *   tickets but cannot be given to a ticket anymore. `DELETE` answers 409
 *   while any ticket uses the type.
 * - Members read; workspace owners and admins manage.
 */

import type { Assert, Covers } from '../internal/covers';
import type { TicketTypeColor } from '../generated/ticket_types/TicketTypeColor';
import type { TicketTypeIcon } from '../generated/ticket_types/TicketTypeIcon';

export type { CreateTicketTypeRequest } from '../generated/ticket_types/CreateTicketTypeRequest';
export type { ListTicketTypesQuery } from '../generated/ticket_types/ListTicketTypesQuery';
export type { ReorderTicketTypesRequest } from '../generated/ticket_types/ReorderTicketTypesRequest';
export type { TicketTypeColor } from '../generated/ticket_types/TicketTypeColor';
export type { TicketTypeIcon } from '../generated/ticket_types/TicketTypeIcon';
export type { TicketTypeResponse } from '../generated/ticket_types/TicketTypeResponse';
export type { UpdateTicketTypeRequest } from '../generated/ticket_types/UpdateTicketTypeRequest';

export const TICKET_TYPE_ICONS = [
  'square-check',
  'bug',
  'sparkles',
  'life-buoy',
  'layers',
  'circle-dot',
  'flag',
  'zap',
  'star',
  'book-open',
  'wrench',
  'shield',
  'rocket',
  'lightbulb',
  'message-circle',
  'file-text',
  'target',
  'list-checks',
  'git-branch',
  'triangle-alert',
  'palette',
  'package',
  'megaphone',
  'flask-conical',
] as const satisfies readonly TicketTypeIcon[];
type _IconsComplete = Assert<Covers<TicketTypeIcon, typeof TICKET_TYPE_ICONS>>;

/** Token names (`--blue`, `--blue-tint`, …), never hex. */
export const TICKET_TYPE_COLORS = [
  'neutral',
  'blue',
  'green',
  'amber',
  'red',
  'purple',
  'pink',
] as const satisfies readonly TicketTypeColor[];
type _ColorsComplete = Assert<Covers<TicketTypeColor, typeof TICKET_TYPE_COLORS>>;

/** 0 = an ordinary ticket, 1 = an epic-like parent of ordinary tickets. */
export type TicketTypeHierarchyLevel = 0 | 1;
