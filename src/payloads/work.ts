import type { ActivityAction } from './activity';

export interface WorkspaceStats {
  open_tickets: number;
  in_progress_tickets: number;
  total_projects: number;
}

export interface ActivityEntry {
  actor_initials: string;
  actor_name: string | null;
  action: string;
  action_label: string;
  resource_type: string;
  resource_id: string | null;
  details: Record<string, unknown> | null;
  created_at: string;
}

export interface MyTicketEntry {
  id: string;
  project_id: string;
  title: string;
  description: string;
  status: string;
  priority: string;
  assignee_id: string | null;
  reporter_id: string;
  sla_policy_id: string | null;
  triage_category: string | null;
  triage_reason: string | null;
  triaged_at: string | null;
  first_response_at: string | null;
  resolved_at: string | null;
  sla_breached: boolean;
  created_at: string;
  updated_at: string;
}

// Full ticket response shape returned by `GET /work/tickets`, `POST /work/tickets`,
// and the status/assign mutations. Mirrors `athene::transport::http::handlers::work::TicketDto`.
export type TicketStatus =
  | 'open'
  | 'in_progress'
  | 'in_review'
  | 'blocked'
  | 'done'
  | 'closed';

export type TicketPriority = 'low' | 'medium' | 'high' | 'critical';

export type TicketType = 'task' | 'bug' | 'feature' | 'support' | 'epic';

export interface TicketResponse {
  id: string;
  project_id: string;
  /**
   * Per-project monotonic counter. Combined with the project's `key` it
   * produces the human-readable ticket key (e.g. `WEB-128`). Backfilled
   * for legacy tickets by migration 0007.
   */
  sequence: number;
  title: string;
  description: string;
  ticket_type: TicketType;
  status: TicketStatus;
  priority: TicketPriority;
  assignee_id: string | null;
  reporter_id: string;
  sla_policy_id: string | null;
  /**
   * Set when this ticket is a subtask. The subtask tree is built from this
   * field, not from the link table, so a subtask has exactly one parent by
   * construction and the server rejects cycles.
   */
  parent_ticket_id: string | null;
  sprint_id: string | null;
  milestone_id: string | null;
  /** Planning estimate, 0..1000. */
  story_points: number | null;
  triage_category: string | null;
  triage_reason: string | null;
  triaged_at: string | null;
  first_response_at: string | null;
  resolved_at: string | null;
  due_date: string | null;
  sla_breached: boolean;
  /**
   * Custom field values keyed by `FieldDefinition.key`. Server-validated
   * against the workspace's definitions before persistence; unknown keys
   * are dropped server-side. Always a plain object — `{}` when empty.
   */
  custom_values: Record<string, unknown>;
  /**
   * Position within the board column, ordered inside (project, status).
   * `null` for tickets written before ranking existed — sort a column by
   * this value and place the nulls last, newest first.
   */
  board_rank: number | null;
  created_at: string;
  updated_at: string;
}

/**
 * Why an inbox entry reached the reader.
 *
 * Ordered by how directly it concerns them: when several apply the strongest
 * wins, because "assigned to you" explains an entry better than "you watch
 * this" even when both are true.
 */
export type InboxReason = 'assigned' | 'reported' | 'watching';

/**
 * One inbox entry: something that happened, on a ticket the reader is
 * involved in.
 *
 * Shaped like an activity event with the ticket attached, so the same
 * renderer serves both surfaces. The reader's own actions are never
 * included — an inbox full of what you just did buries what somebody else
 * did.
 */
export interface InboxEntryResponse {
  /** The activity event's id. */
  id: string;
  actor_id: string | null;
  action: ActivityAction;
  detail: string | null;
  metadata: Record<string, unknown> | null;
  /** RFC3339. */
  created_at: string;
  reason: InboxReason;
  ticket: TicketResponse;
}

export interface TicketCommentResponse {
  id: string;
  ticket_id: string;
  author_id: string;
  body: string;
  /** RFC3339 timestamp. */
  created_at: string;
}

export interface AddTicketCommentRequest {
  body: string;
}

export interface ListTicketsQuery {
  project_id?: string;
}

export interface CreateTicketRequest {
  project_id: string;
  title: string;
  description: string;
  priority: string;
  assignee_id?: string | null;
  sla_policy_id?: string | null;
  /** RFC3339 timestamp. */
  due_date?: string | null;
  /**
   * Makes the new ticket a subtask of this one, in the same request — so a
   * subtask is never briefly a top-level ticket. The parent must exist.
   */
  parent_ticket_id?: string | null;
  /**
   * Custom field values keyed by `FieldDefinition.key`. Optional — defaults
   * to `{}`. Server validates against the project's workspace definitions
   * and returns 400 for type mismatches or missing required fields.
   */
  custom_values?: Record<string, unknown>;
}

export interface UpdateTicketStatusRequest {
  status: string;
}

/**
 * Content patch for a ticket: `PATCH /work/tickets/:id`.
 *
 * Only the keys present in the request are touched. For the nullable fields
 * that matters twice over: **omit** a key to leave it alone, send `null` to
 * clear it. Sending `{ title }` alone will not wipe the due date; sending
 * `{ due_date: null }` will.
 *
 * Status and assignee are deliberately absent — they have their own
 * endpoints because they carry workflow side effects (SLA stamps,
 * notifications) that a content edit should not trigger.
 */
export interface UpdateTicketRequest {
  title?: string;
  description?: string;
  ticket_type?: TicketType;
  priority?: TicketPriority;
  /** RFC3339 timestamp, or `null` to clear. */
  due_date?: string | null;
  story_points?: number | null;
  parent_ticket_id?: string | null;
}

/**
 * One patch applied to a selection of tickets.
 *
 * The same patch shape as a single edit, so a field the one accepts cannot
 * quietly be ignored by the other.
 */
export interface BulkUpdateTicketsRequest {
  ticket_ids: string[];
  patch: UpdateTicketRequest;
  /**
   * Taken apart from the patch, and applied through the status endpoint's
   * own path: a status change stamps SLA timers and clears a resolution
   * date, and writing it as a plain field would skip all of that.
   */
  status?: TicketStatus;
}

/** Why one ticket in a bulk edit did not move. */
export interface BulkUpdateFailure {
  ticket_id: string;
  message: string;
}

/**
 * What a bulk edit did, per ticket.
 *
 * Deliberately not all-or-nothing: a selection of fifty where one has been
 * deleted under the user moves the other forty-nine and names the one that
 * did not. Both halves matter — `updated` replaces what the client holds,
 * `failed` is what to tell the user about.
 */
export interface BulkUpdateTicketsResponse {
  updated: TicketResponse[];
  failed: BulkUpdateFailure[];
}

export interface MoveTicketRequest {
  /** Target column. Same vocabulary as `UpdateTicketStatusRequest.status`. */
  status: string;
  /**
   * Zero-based position within the target column, counted after the ticket
   * has been removed from wherever it was. Clamped server-side, so a stale
   * view lands the card at an end instead of failing.
   */
  index: number;
}

export interface UpdateTicketCustomValuesRequest {
  /** Replace the ticket's custom_values bucket wholesale. Server validates
   *  the shape against the workspace's field definitions before saving. */
  custom_values: Record<string, unknown>;
}

export interface UpdateProjectCustomValuesRequest {
  /** Replace the project's custom_values bucket wholesale. Validated
   *  against the workspace's `applies_to = project` definitions. */
  custom_values: Record<string, unknown>;
}

export interface AssignTicketRequest {
  assignee_id: string | null;
}

export type ProjectHealth = 'on_track' | 'at_risk' | 'blocked';
export type ProjectStatus = 'active' | 'archived';

export interface ProjectResponse {
  id: string;
  /**
   * The workspace the project belongs to. `null` for projects created before
   * projects carried a workspace — those are invisible to workspace-scoped
   * features (the activity feed, custom field definitions).
   */
  workspace_id: string | null;
  key: string;
  name: string;
  description: string | null;
  /** Hex color `#RRGGBB`. When null the frontend derives a stable colour from the key. */
  color: string | null;
  health: string;
  /** 0..1 — completion ratio, manually set or derived from tickets. */
  progress: number;
  lead_id: string | null;
  status: string;
  /**
   * Values for the workspace's `applies_to = project` field definitions,
   * keyed by each definition's stable `key`. Always an object; `{}` when
   * the workspace defines no project fields.
   */
  custom_values: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface CreateProjectRequest {
  key: string;
  name: string;
  /**
   * Always send this. A project without a workspace records no activity and
   * gets none of the workspace's custom fields; the field is optional only so
   * older clients keep compiling.
   */
  workspace_id?: string | null;
  description?: string | null;
  /** Optional hex color `#RRGGBB` — overrides the deterministic palette. */
  color?: string | null;
}

/**
 * A patch for a project.
 *
 * Every field is optional and only the ones present are applied, so a patch
 * that mentions nothing is rejected rather than treated as "clear
 * everything". `key` is not editable: it prefixes every ticket ever raised
 * in the project.
 */
export interface UpdateProjectRequest {
  name?: string;
  /** `null` clears the description. */
  description?: string | null;
  /** `#RRGGBB`, or `null` to fall back to the derived palette colour. */
  color?: string | null;
  health?: ProjectHealth;
  status?: ProjectStatus;
  /** `null` leaves the project without a lead. */
  lead_id?: string | null;
}

export interface UpcomingEntry {
  type: 'ticket' | 'milestone';
  id: string;
  title: string;
  due_date: string;
  project_key?: string;
}

export interface MilestoneResponse {
  id: string;
  title: string;
  description: string | null;
  due_date: string;
  created_by: string;
  created_at: string;
}

export interface DashboardWidgetLayout {
  widget_id: string;
  x: number;
  y: number;
  cols: number;
  rows: number;
  visible: boolean;
}

// ============================================================
// Design extensions: Sprints, Labels, TicketLinks, Watchers
// (synced with rust crate `work.rs`)
// ============================================================

export type SprintStatus = 'upcoming' | 'active' | 'completed';

export interface SprintResponse {
  id: string;
  project_id: string;
  name: string;
  goal: string | null;
  /** ISO date (YYYY-MM-DD) */
  start_date: string;
  end_date: string;
  goal_points: number | null;
  status: SprintStatus;
  created_at: string;
  updated_at: string;
}

export interface CreateSprintRequest {
  name: string;
  goal?: string | null;
  start_date: string;
  end_date: string;
  goal_points?: number | null;
}

export interface AssignTicketsRequest {
  ticket_ids: string[];
}

/**
 * Where a completed sprint's unfinished tickets go: another sprint of the
 * same project that is not completed, or — absent or `null` — the backlog.
 */
export interface CompleteSprintRequest {
  move_open_to?: string | null;
}

export interface SprintBurnDownResponse {
  sprint_id: string;
  total_points: number;
  done_points: number;
  total_tickets: number;
  done_tickets: number;
  completion_pct: number;
}

export interface LabelResponse {
  id: string;
  workspace_id: string;
  name: string;
  color: string | null;
  created_at: string;
}

export interface CreateLabelRequest {
  name: string;
  /** `#RRGGBB` */
  color?: string | null;
}

/**
 * Rename and/or recolour a label. An absent key leaves that part as it is;
 * `color: null` clears the colour back to the theme's default.
 */
export interface UpdateLabelRequest {
  name?: string;
  /** `#RRGGBB`, or `null` to clear. */
  color?: string | null;
}

/** One label on one ticket. */
export interface LabelAssignmentResponse {
  ticket_id: string;
  label_id: string;
}

export interface SetLabelsRequest {
  label_ids: string[];
  workspace_id?: string | null;
}

export type LinkRelation =
  | 'blocks'
  | 'blocked_by'
  | 'related'
  | 'duplicates'
  | 'duplicated_by'
  | 'child_of'
  | 'parent_of';

/**
 * One link on a ticket, with the ticket on the other end already resolved.
 *
 * `relation` reads from the perspective of the ticket that was asked about:
 * `blocks` means *this* ticket blocks `ticket`. The server writes the
 * symmetric counterpart too, so the other ticket shows the inverse without a
 * second request.
 *
 * Parent/child hierarchy does **not** come through here — it lives on
 * `TicketResponse.parent_ticket_id`.
 */
export interface TicketLinkResponse {
  relation: LinkRelation;
  ticket: TicketResponse;
  created_at: string;
  created_by: string | null;
}

export interface CreateTicketLinkRequest {
  to_ticket_id: string;
  relation: LinkRelation;
}

export interface TicketWatcherResponse {
  user_id: string;
  /** `null` when the user record is gone; fall back to the id. */
  display_name: string | null;
  email: string | null;
  added_at: string;
}

/**
 * The watcher list plus whether the caller is on it, so the list and the
 * watch toggle come from one request instead of two.
 */
export interface TicketWatcherListResponse {
  watchers: TicketWatcherResponse[];
  watching: boolean;
}

/** Response of the watch / unwatch toggle. */
export interface WatchStateResponse {
  watching: boolean;
}
