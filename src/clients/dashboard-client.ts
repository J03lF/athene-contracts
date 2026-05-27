import type {
  DashboardOverviewResponse,
  FocusTab,
  FocusTicket,
  KpisResponse,
} from '../payloads/dashboard';
import type { AtheneHttpClient } from '../http';

export class DashboardApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  overview(workspaceId: string): Promise<DashboardOverviewResponse> {
    return this.http.get(`/dashboard/overview?workspace_id=${encodeURIComponent(workspaceId)}`);
  }

  kpis(): Promise<KpisResponse> {
    return this.http.get(`/dashboard/kpis`);
  }

  focus(tab: FocusTab = 'today', limit = 8): Promise<FocusTicket[]> {
    return this.http.get(`/dashboard/focus?tab=${tab}&limit=${limit}`);
  }
}
