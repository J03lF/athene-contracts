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
  due_date: string | null;
  sla_breached: boolean;
  /**
   * Custom field values keyed by `FieldDefinition.key`. Server-validated
   * against the workspace's definitions before persistence; unknown keys
   * are dropped server-side. Always a plain object — `{}` when empty.
   */
  custom_values: Record<string, unknown>;
  created_at: string;
  updated_at: string;
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
   * Custom field values keyed by `FieldDefinition.key`. Optional — defaults
   * to `{}`. Server validates against the project's workspace definitions
   * and returns 400 for type mismatches or missing required fields.
   */
  custom_values?: Record<string, unknown>;
}

export interface UpdateTicketStatusRequest {
  status: string;
}

export interface UpdateTicketCustomValuesRequest {
  /** Replace the ticket's custom_values bucket wholesale. Server validates
   *  the shape against the workspace's field definitions before saving. */
  custom_values: Record<string, unknown>;
}

export interface AssignTicketRequest {
  assignee_id: string | null;
}

export type ProjectHealth = 'on_track' | 'at_risk' | 'blocked';

export interface ProjectResponse {
  id: string;
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
  created_at: string;
  updated_at: string;
}

export interface CreateProjectRequest {
  key: string;
  name: string;
  description?: string | null;
  /** Optional hex color `#RRGGBB` — overrides the deterministic palette. */
  color?: string | null;
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

export interface TicketLinkResponse {
  from_ticket_id: string;
  to_ticket_id: string;
  relation: LinkRelation;
  created_at: string;
  created_by: string | null;
}

export interface CreateTicketLinkRequest {
  to_ticket_id: string;
  relation: LinkRelation;
}

export interface WatcherResponse {
  ticket_id: string;
  user_id: string;
  added_at: string;
}

export interface AddWatcherRequest {
  user_id: string;
}
