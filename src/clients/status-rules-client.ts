import type { AthenosHttpClient } from '../http';
import type {
  SetStatusRulesRequest,
  SetStatusRulesResponse,
  WipCountsResponse,
  WorkflowRulesResponse,
} from '../payloads/status-rules';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

/**
 * The rules on a workspace's statuses (W4-G): approvals and WIP limits.
 * Reading is every member's; setting them, a project's included, is
 * `workspace.settings.edit` (owners and admins).
 */
export class StatusRulesApiClient {
  constructor(private readonly http: AthenosHttpClient) {}

  /** Every rule of the workspace: its own first, then the projects'. */
  list(workspaceId: string): Promise<WorkflowRulesResponse> {
    return this.http
      .get<ApiEnvelope<WorkflowRulesResponse>>(`/work/workspaces/${workspaceId}/statuses/rules`)
      .then(unwrap);
  }

  /**
   * One status's rules at one place (`project_id` absent: the workspace).
   * `approval` / `wip_limit` absent keeps, `null` clears. `409 ATH-RES-003`
   * with `details.current` when `version` is stale.
   */
  set(
    workspaceId: string,
    statusId: string,
    body: SetStatusRulesRequest,
  ): Promise<SetStatusRulesResponse> {
    return this.http
      .put<ApiEnvelope<SetStatusRulesResponse>>(
        `/work/workspaces/${workspaceId}/statuses/${statusId}/rules`,
        body,
      )
      .then(unwrap);
  }

  /** The counts beside the WIP limits in effect, for a board's "3/5". */
  wip(workspaceId: string, projectId?: string): Promise<WipCountsResponse> {
    const query = projectId ? `?project_id=${encodeURIComponent(projectId)}` : '';
    return this.http
      .get<ApiEnvelope<WipCountsResponse>>(`/work/workspaces/${workspaceId}/wip${query}`)
      .then(unwrap);
  }
}
