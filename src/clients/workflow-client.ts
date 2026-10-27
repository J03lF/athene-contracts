import type { AtheneHttpClient } from '../http';
import type {
  CreateWorkflowStatusRequest,
  DeleteWorkflowStatusRequest,
  DeleteWorkflowStatusResponse,
  ProjectHiddenStatusesResponse,
  ReorderWorkflowStatusesRequest,
  SetHiddenStatusesRequest,
  UpdateWorkflowStatusRequest,
  WorkflowStatusResponse,
  WorkspaceWorkflowResponse,
} from '../payloads/workflow';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

/**
 * A workspace's statuses (W4-F). Reading is every member's; creating,
 * editing, ordering, deleting and hiding per project is
 * `workspace.settings.edit` (owners and admins).
 */
export class WorkflowApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  /** Every status by position, archived ones included; `withUsage` adds `ticket_count`. */
  list(workspaceId: string, withUsage = false): Promise<WorkspaceWorkflowResponse> {
    const query = withUsage ? '?with_usage=true' : '';
    return this.http
      .get<ApiEnvelope<WorkspaceWorkflowResponse>>(
        `/work/workspaces/${workspaceId}/statuses${query}`,
      )
      .then(unwrap);
  }

  create(workspaceId: string, body: CreateWorkflowStatusRequest): Promise<WorkflowStatusResponse> {
    return this.http
      .post<ApiEnvelope<WorkflowStatusResponse>>(`/work/workspaces/${workspaceId}/statuses`, body)
      .then(unwrap);
  }

  /** Field-level with `expected`: `409 ATH-RES-003` with `details.current`. */
  update(
    workspaceId: string,
    statusId: string,
    body: UpdateWorkflowStatusRequest,
  ): Promise<WorkflowStatusResponse> {
    return this.http
      .patch<ApiEnvelope<WorkflowStatusResponse>>(
        `/work/workspaces/${workspaceId}/statuses/${statusId}`,
        body,
      )
      .then(unwrap);
  }

  /** Every status id of the workspace, in the new order. */
  reorder(workspaceId: string, body: ReorderWorkflowStatusesRequest): Promise<WorkspaceWorkflowResponse> {
    return this.http
      .put<ApiEnvelope<WorkspaceWorkflowResponse>>(
        `/work/workspaces/${workspaceId}/statuses/order`,
        body,
      )
      .then(unwrap);
  }

  /**
   * Delete a workspace's own status. Without `move_to`, a status some
   * ticket holds is `409` (`details.reason` `status_in_use`,
   * `details.tickets`); with it, its tickets move in the same statement.
   */
  remove(
    workspaceId: string,
    statusId: string,
    body: DeleteWorkflowStatusRequest = {},
  ): Promise<DeleteWorkflowStatusResponse> {
    return this.http
      .post<ApiEnvelope<DeleteWorkflowStatusResponse>>(
        `/work/workspaces/${workspaceId}/statuses/${statusId}/delete`,
        body,
      )
      .then(unwrap);
  }

  /** The keys hidden in one project of the workspace, all of them (`[]` shows every status). */
  setHidden(workspaceId: string, body: SetHiddenStatusesRequest): Promise<ProjectHiddenStatusesResponse> {
    return this.http
      .put<ApiEnvelope<ProjectHiddenStatusesResponse>>(
        `/work/workspaces/${workspaceId}/statuses/hidden`,
        body,
      )
      .then(unwrap);
  }
}
