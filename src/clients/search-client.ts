import type { AthenosHttpClient, RequestOptions } from '../http';
import type { SearchTicketsQuery, TicketSearchResponse } from '../payloads/search';

interface ApiEnvelope<T> {
  data: T;
}

/** Ticket and doc search in one workspace (3.9). */
export class SearchApiClient {
  constructor(private readonly http: AthenosHttpClient) {}

  /**
   * Tickets of the workspace matching `query.q` (full text, typo-tolerant
   * titles, `WEB-12` keys), with the server's filters and facet counts.
   * 403 for a non-member.
   */
  searchTickets(
    workspaceId: string,
    query: SearchTicketsQuery = {},
    options: RequestOptions = {}
  ): Promise<TicketSearchResponse> {
    const params = new URLSearchParams();
    if (query.q) params.set('q', query.q);
    if (query.project_id) params.set('project_id', query.project_id);
    if (query.status) params.set('status', query.status);
    if (query.type) params.set('type', query.type);
    if (query.assignee) params.set('assignee', query.assignee);
    if (query.label) params.set('label', query.label);
    if (query.archived) params.set('archived', 'true');
    if (query.limit !== undefined) params.set('limit', String(query.limit));
    if (query.doc_limit !== undefined) params.set('doc_limit', String(query.doc_limit));
    const qs = params.toString();
    return this.http
      .get<
        ApiEnvelope<TicketSearchResponse>
      >(`/work/workspaces/${workspaceId}/search${qs ? `?${qs}` : ''}`, options)
      .then((envelope) => envelope.data);
  }
}
