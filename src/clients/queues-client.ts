import type { AtheneHttpClient } from '../http';
import type {
  CreateQueueRequest,
  DeleteQueueResponse,
  HandBackTicketRequest,
  MyQueuesResponse,
  QueueResponse,
  TakeTicketRequest,
  UpdateQueueRequest,
  WorkspaceQueuesResponse,
} from '../payloads/queues';
import type { TicketResponse } from '../payloads/work';
import type { TicketWriteResponse } from '../payloads/status-rules';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

/**
 * A workspace's queues and the hand-offs on tickets (custom roles, phase
 * C1). Reading is every member's; editing queues is
 * `workspace.settings.edit` (owners and admins). Taking and handing back
 * need `ticket.take` and the queue's target role (or governing the
 * workspace).
 */
export class QueuesApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  /** Every queue with its waiting count, `mine` and `can_take`. */
  list(workspaceId: string): Promise<WorkspaceQueuesResponse> {
    return this.http
      .get<ApiEnvelope<WorkspaceQueuesResponse>>(`/work/workspaces/${workspaceId}/queues`)
      .then(unwrap);
  }

  /** The queues the caller works, with their waiting tickets. */
  mine(workspaceId: string): Promise<MyQueuesResponse> {
    return this.http
      .get<ApiEnvelope<MyQueuesResponse>>(`/work/workspaces/${workspaceId}/queues/mine`)
      .then(unwrap);
  }

  create(workspaceId: string, body: CreateQueueRequest): Promise<QueueResponse> {
    return this.http
      .post<ApiEnvelope<QueueResponse>>(`/work/workspaces/${workspaceId}/queues`, body)
      .then(unwrap);
  }

  /** Field-level with `expected`: `409 ATH-RES-003` with `details.current`. */
  update(workspaceId: string, queueId: string, body: UpdateQueueRequest): Promise<QueueResponse> {
    return this.http
      .patch<ApiEnvelope<QueueResponse>>(
        `/work/workspaces/${workspaceId}/queues/${queueId}`,
        body,
      )
      .then(unwrap);
  }

  remove(workspaceId: string, queueId: string): Promise<DeleteQueueResponse> {
    return this.http
      .delete<ApiEnvelope<DeleteQueueResponse>>(
        `/work/workspaces/${workspaceId}/queues/${queueId}`,
      )
      .then(unwrap);
  }

  /**
   * "Nehmen": the caller becomes the assignee if nobody is. `409
   * ATH-RES-005` names who was faster (`details.taken_by`), `409
   * ATH-RES-006` says the ticket left the queue. Taking fills a WIP limit
   * per person on the ticket's status (W4-K): `warnings` in the envelope,
   * or `409` `wip_limit_reached` (owners and admins may send
   * `wip_override`).
   */
  take(ticketId: string, body: TakeTicketRequest = {}): Promise<TicketWriteResponse> {
    return this.http.post<TicketWriteResponse>(`/work/tickets/${ticketId}/take`, body);
  }

  /** "Zurückschicken": `in_progress`, to `handoff_from`, with a comment. */
  handBack(ticketId: string, body: HandBackTicketRequest): Promise<TicketResponse> {
    return this.http
      .post<ApiEnvelope<TicketResponse>>(`/work/tickets/${ticketId}/hand-back`, body)
      .then(unwrap);
  }
}
