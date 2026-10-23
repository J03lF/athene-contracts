/**
 * Ticket-type wire shapes — the configurable kinds of ticket per workspace.
 * Mirrored by hand in `rust/athene-contracts/src/ticket_types.rs`.
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
] as const;
export type TicketTypeIcon = (typeof TICKET_TYPE_ICONS)[number];

/** Token names (`--blue`, `--blue-tint`, …), never hex. */
export const TICKET_TYPE_COLORS = [
  'neutral',
  'blue',
  'green',
  'amber',
  'red',
  'purple',
  'pink',
] as const;
export type TicketTypeColor = (typeof TICKET_TYPE_COLORS)[number];

/** 0 = an ordinary ticket, 1 = an epic-like parent of ordinary tickets. */
export type TicketTypeHierarchyLevel = 0 | 1;

export interface TicketTypeResponse {
  id: string;
  workspace_id: string;
  /** Stable; what tickets store in `ticket_type`. */
  key: string;
  name: string;
  description: string | null;
  /** Markdown prefilled into a new ticket's empty description; `null` = none. */
  description_template: string | null;
  icon: TicketTypeIcon;
  color: TicketTypeColor;
  position: number;
  is_default: boolean;
  hierarchy_level: TicketTypeHierarchyLevel;
  /** `null` while live. */
  archived_at: string | null;
  /** Tickets of the workspace that carry this type. */
  ticket_count: number;
  created_at: string;
  updated_at: string;
}

export interface ListTicketTypesQuery {
  /** Include archived types (settings page). Default: live types only. */
  include_archived?: boolean;
}

export interface CreateTicketTypeRequest {
  name: string;
  /** Absent = derived from the name (`"Change request"` → `change_request`). */
  key?: string;
  description?: string | null;
  description_template?: string | null;
  /** Default `circle-dot`. */
  icon?: TicketTypeIcon;
  /** Default `neutral`. */
  color?: TicketTypeColor;
  /** Default 0. */
  hierarchy_level?: TicketTypeHierarchyLevel;
  /** `true` makes it the workspace default (the previous one stops being it). */
  is_default?: boolean;
}

/** An absent key leaves that part as it is; `null` clears `description` or `description_template`. */
export interface UpdateTicketTypeRequest {
  name?: string;
  description?: string | null;
  description_template?: string | null;
  icon?: TicketTypeIcon;
  color?: TicketTypeColor;
  hierarchy_level?: TicketTypeHierarchyLevel;
  /** Only `true` is meaningful: make this the default. */
  is_default?: boolean;
  /** `true` archives, `false` restores. */
  archived?: boolean;
}

export interface ReorderTicketTypesRequest {
  /** Every live and archived type id of the workspace, in the new order. */
  ordered_ids: string[];
}
