import type {
  DashboardOverviewResponse,
  FocusTab,
  FocusTicket,
  KpisResponse,
} from '../payloads/dashboard';
import type { AthenosHttpClient } from '../http';

export interface DashboardScopeQuery {
  /** Only this workspace (403 for a non-member). */
  workspace_id?: string;
}

export interface FocusQuery extends DashboardScopeQuery {
  tab?: FocusTab;
  limit?: number;
}

export class DashboardApiClient {
  constructor(private readonly http: AthenosHttpClient) {}

  overview(workspaceId: string): Promise<DashboardOverviewResponse> {
    return this.http.get(`/dashboard/overview?workspace_id=${encodeURIComponent(workspaceId)}`);
  }

  /**
   * The caller's tiles. With `workspace_id`, counted in that workspace only
   * (403 for a non-member); without, across every workspace — pass it.
   */
  kpis(query: DashboardScopeQuery = {}): Promise<KpisResponse> {
    const qs = query.workspace_id
      ? `?workspace_id=${encodeURIComponent(query.workspace_id)}`
      : '';
    return this.http.get(`/dashboard/kpis${qs}`);
  }

  /**
   * `focus('today', 8)` still works; `focus({ tab, limit, workspace_id })`
   * scopes the list to one workspace (403 for a non-member).
   */
  focus(arg: FocusTab | FocusQuery = 'today', limit = 8): Promise<FocusTicket[]> {
    const q: FocusQuery = typeof arg === 'string' ? { tab: arg, limit } : arg;
    const params = new URLSearchParams();
    params.set('tab', q.tab ?? 'today');
    params.set('limit', String(q.limit ?? 8));
    if (q.workspace_id) params.set('workspace_id', q.workspace_id);
    return this.http.get(`/dashboard/focus?${params.toString()}`);
  }
}
