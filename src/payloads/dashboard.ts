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
  key: string;
  title: string;
  status: string;
  priority: string;
  assignee_id: string | null;
  project_id: string;
  due_date: string | null;
}

export interface DashboardOverviewResponse {
  kpis: KpisResponse;
  focus: FocusTicket[];
  recent_activity: ActivityEventResponse[];
  on_track_text: string;
}

export type FocusTab = 'today' | 'assigned' | 'review';
