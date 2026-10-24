import type { AtheneHttpClient } from '../http';
import type {
  DeleteIntelDataResponse,
  RecordDecisionsRequest,
  RecordDecisionsResponse,
  TicketCheckRequest,
  TicketCheckResponse,
} from '../payloads/ticket-check';

interface ApiEnvelope<T> {
  data: T;
}

const unwrap = <T>(envelope: ApiEnvelope<T>): T => envelope.data;

/** The ticket check: check a draft, record decisions, forget what was learned. */
export class TicketCheckApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  /** Checks a draft; stores nothing. */
  check(workspaceId: string, body: TicketCheckRequest): Promise<TicketCheckResponse> {
    return this.http
      .post<ApiEnvelope<TicketCheckResponse>>(`/work/workspaces/${workspaceId}/ticket-check`, body)
      .then(unwrap);
  }

  /** What the person did with suggestions (signed tokens, at most 50). */
  recordDecisions(
    workspaceId: string,
    body: RecordDecisionsRequest
  ): Promise<RecordDecisionsResponse> {
    return this.http
      .post<
        ApiEnvelope<RecordDecisionsResponse>
      >(`/work/workspaces/${workspaceId}/intel/decisions`, body)
      .then(unwrap);
  }

  /** Deletes what the workspace learned (owners and admins). */
  deleteData(workspaceId: string): Promise<DeleteIntelDataResponse> {
    return this.http
      .post<ApiEnvelope<DeleteIntelDataResponse>>(`/work/workspaces/${workspaceId}/intel/data/delete`, {})
      .then(unwrap);
  }
}
