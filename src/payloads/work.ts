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
