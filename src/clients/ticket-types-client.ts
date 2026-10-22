import type {
  CreateTicketTypeRequest,
  ListTicketTypesQuery,
  TicketTypeResponse,
  UpdateTicketTypeRequest,
} from '../payloads/ticket-types';
import type { AtheneHttpClient } from '../http';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

/**
 * Ticket-types API surface — the configurable kinds of ticket per workspace.
 *
 * Members read; workspace owners and admins manage. A workspace or type
 * the caller cannot see answers 403, whether or not it exists.
 */
export class TicketTypesApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  /** Ordered by `position`. Live types only unless `include_archived`. */
  list(workspaceId: string, query: ListTicketTypesQuery = {}): Promise<TicketTypeResponse[]> {
    const qs = query.include_archived ? '?include_archived=true' : '';
    return this.http
      .get<ApiEnvelope<TicketTypeResponse[]>>(`/work/workspaces/${workspaceId}/ticket-types${qs}`)
      .then(unwrap);
  }

  create(workspaceId: string, body: CreateTicketTypeRequest): Promise<TicketTypeResponse> {
    return this.http
      .post<ApiEnvelope<TicketTypeResponse>>(`/work/workspaces/${workspaceId}/ticket-types`, body)
      .then(unwrap);
  }

  update(id: string, body: UpdateTicketTypeRequest): Promise<TicketTypeResponse> {
    return this.http
      .patch<ApiEnvelope<TicketTypeResponse>>(`/work/ticket-types/${id}`, body)
      .then(unwrap);
  }

  archive(id: string): Promise<TicketTypeResponse> {
    return this.update(id, { archived: true });
  }

  unarchive(id: string): Promise<TicketTypeResponse> {
    return this.update(id, { archived: false });
  }

  /** Returns every type (archived included) in the new order. */
  reorder(workspaceId: string, orderedIds: string[]): Promise<TicketTypeResponse[]> {
    return this.http
      .post<ApiEnvelope<TicketTypeResponse[]>>(
        `/work/workspaces/${workspaceId}/ticket-types/reorder`,
        { ordered_ids: orderedIds },
      )
      .then(unwrap);
  }

  /** 409 while any ticket uses the type — archive it instead. */
  delete(id: string): Promise<void> {
    return this.http.delete(`/work/ticket-types/${id}`).then(() => undefined);
  }
}
