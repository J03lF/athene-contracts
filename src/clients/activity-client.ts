import type { ActivityEventResponse } from '../payloads/activity';
import type { AtheneHttpClient } from '../http';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

export class ActivityApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  /** Workspace-wide feed, newest first. */
  listWorkspace(workspaceId: string, limit = 20): Promise<ActivityEventResponse[]> {
    return this.http
      .get<ApiEnvelope<ActivityEventResponse[]>>(
        `/work/workspaces/${workspaceId}/activity?limit=${limit}`,
      )
      .then(unwrap);
  }

  /** Everything that happened to one ticket, newest first. */
  listForTicket(ticketId: string, limit = 20): Promise<ActivityEventResponse[]> {
    return this.http
      .get<ApiEnvelope<ActivityEventResponse[]>>(
        `/work/tickets/${ticketId}/activity?limit=${limit}`,
      )
      .then(unwrap);
  }
}
