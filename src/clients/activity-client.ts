import type { ActivityEventResponse } from '../payloads/activity';
import type { AtheneHttpClient } from '../http';

export class ActivityApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  listWorkspace(workspaceId: string, limit = 20): Promise<ActivityEventResponse[]> {
    return this.http.get(`/work/workspaces/${workspaceId}/activity?limit=${limit}`);
  }

  listForTicket(ticketId: string, limit = 20): Promise<ActivityEventResponse[]> {
    return this.http.get(`/work/tickets/${ticketId}/activity?limit=${limit}`);
  }
}
