import type {
  AcceptFieldSuggestionRequest,
  AcceptFieldSuggestionResponse,
  CreateFieldAssignmentRequest,
  CreateFieldSuggestionRequest,
  FieldAssignmentResponse,
  FieldSuggestionResponse,
  FieldSuggestionStatus,
  UpdateFieldAssignmentRequest,
} from '../payloads/field-assignments';
import type { TicketResponse } from '../payloads/work';
import type { AthenosHttpClient, RequestOptions } from '../http';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

/**
 * Where ticket fields apply (custom fields CF1): the workspace's rules,
 * fields of one ticket, and suggestions.
 *
 * - Rules: `fields.manage`; a rule for one project also with
 *   `project.edit` there.
 * - Attach/detach on a ticket: `field.attach` (the tick "Tickets
 *   bearbeiten").
 * - Suggest: `field.suggest` on the ticket; accept/decline like a rule.
 */
export class FieldAssignmentsApiClient {
  constructor(private readonly http: AthenosHttpClient) {}

  list(workspaceId: string, options: RequestOptions = {}): Promise<FieldAssignmentResponse[]> {
    return this.http
      .get<ApiEnvelope<FieldAssignmentResponse[]>>(
        `/work/workspaces/${workspaceId}/field-assignments`,
        options,
      )
      .then(unwrap);
  }

  create(
    workspaceId: string,
    body: CreateFieldAssignmentRequest,
  ): Promise<FieldAssignmentResponse> {
    return this.http
      .post<ApiEnvelope<FieldAssignmentResponse>>(
        `/work/workspaces/${workspaceId}/field-assignments`,
        body,
      )
      .then(unwrap);
  }

  update(id: string, body: UpdateFieldAssignmentRequest): Promise<FieldAssignmentResponse> {
    return this.http
      .patch<ApiEnvelope<FieldAssignmentResponse>>(`/work/field-assignments/${id}`, body)
      .then(unwrap);
  }

  delete(id: string): Promise<void> {
    return this.http.delete(`/work/field-assignments/${id}`).then(() => undefined);
  }

  /** Hang an active ticket field onto this ticket alone. */
  attach(ticketId: string, fieldId: string): Promise<TicketResponse> {
    return this.http
      .post<ApiEnvelope<TicketResponse>>(`/work/tickets/${ticketId}/fields`, {
        field_id: fieldId,
      })
      .then(unwrap);
  }

  /** Take it off again; with a value only with `clear` (which empties it). */
  detach(ticketId: string, fieldId: string, clear = false): Promise<TicketResponse> {
    return this.http
      .delete<ApiEnvelope<TicketResponse>>(
        `/work/tickets/${ticketId}/fields/${fieldId}${clear ? '?clear=true' : ''}`,
      )
      .then(unwrap);
  }

  suggestions(
    workspaceId: string,
    status: FieldSuggestionStatus = 'open',
    options: RequestOptions = {},
  ): Promise<FieldSuggestionResponse[]> {
    return this.http
      .get<ApiEnvelope<FieldSuggestionResponse[]>>(
        `/work/workspaces/${workspaceId}/field-suggestions?status=${status}`,
        options,
      )
      .then(unwrap);
  }

  suggest(
    workspaceId: string,
    body: CreateFieldSuggestionRequest,
  ): Promise<FieldSuggestionResponse> {
    return this.http
      .post<ApiEnvelope<FieldSuggestionResponse>>(
        `/work/workspaces/${workspaceId}/field-suggestions`,
        body,
      )
      .then(unwrap);
  }

  accept(
    id: string,
    body: AcceptFieldSuggestionRequest = {},
  ): Promise<AcceptFieldSuggestionResponse> {
    return this.http
      .post<ApiEnvelope<AcceptFieldSuggestionResponse>>(
        `/work/field-suggestions/${id}/accept`,
        body,
      )
      .then(unwrap);
  }

  decline(id: string): Promise<FieldSuggestionResponse> {
    return this.http
      .post<ApiEnvelope<FieldSuggestionResponse>>(`/work/field-suggestions/${id}/decline`, {})
      .then(unwrap);
  }
}
