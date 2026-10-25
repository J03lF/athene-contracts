import type { AtheneHttpClient } from '../http';
import type {
  AtheneAiAdminResponse,
  AtheneAiWorkspaceResponse,
  EnrollAiWorkspaceRequest,
  UpdateAtheneAiRequest,
} from '../payloads/athene-ai';

interface ApiEnvelope<T> {
  data: T;
}

const unwrap = <T>(envelope: ApiEnvelope<T>): T => envelope.data;

/**
 * Athene AI: a workspace's page and beta enrollment, and the platform
 * control in the admin console. Every admin call answers the whole
 * console state.
 */
export class AtheneAiApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  /** The workspace's Athene AI page (members). */
  workspace(workspaceId: string): Promise<AtheneAiWorkspaceResponse> {
    return this.http
      .get<ApiEnvelope<AtheneAiWorkspaceResponse>>(`/work/workspaces/${workspaceId}/athene-ai`)
      .then(unwrap);
  }

  /** Apply for the beta (owners and admins, `preview` only). */
  requestBeta(workspaceId: string): Promise<AtheneAiWorkspaceResponse> {
    return this.http
      .post<
        ApiEnvelope<AtheneAiWorkspaceResponse>
      >(`/work/workspaces/${workspaceId}/athene-ai/beta`, {})
      .then(unwrap);
  }

  /** Withdraw an open request. */
  withdrawBeta(workspaceId: string): Promise<AtheneAiWorkspaceResponse> {
    return this.http
      .delete<ApiEnvelope<AtheneAiWorkspaceResponse>>(`/work/workspaces/${workspaceId}/athene-ai/beta`)
      .then(unwrap);
  }

  /** The platform control (platform admins). */
  admin(): Promise<AtheneAiAdminResponse> {
    return this.http.get<ApiEnvelope<AtheneAiAdminResponse>>('/admin/athene-ai').then(unwrap);
  }

  updateAdmin(body: UpdateAtheneAiRequest): Promise<AtheneAiAdminResponse> {
    return this.http
      .put<ApiEnvelope<AtheneAiAdminResponse>>('/admin/athene-ai', body)
      .then(unwrap);
  }

  enroll(body: EnrollAiWorkspaceRequest): Promise<AtheneAiAdminResponse> {
    return this.http
      .post<ApiEnvelope<AtheneAiAdminResponse>>('/admin/athene-ai/workspaces', body)
      .then(unwrap);
  }

  removeWorkspace(workspaceId: string): Promise<AtheneAiAdminResponse> {
    return this.http
      .delete<ApiEnvelope<AtheneAiAdminResponse>>(`/admin/athene-ai/workspaces/${workspaceId}`)
      .then(unwrap);
  }

  approve(workspaceId: string): Promise<AtheneAiAdminResponse> {
    return this.http
      .post<
        ApiEnvelope<AtheneAiAdminResponse>
      >(`/admin/athene-ai/requests/${workspaceId}/approve`, {})
      .then(unwrap);
  }

  reject(workspaceId: string): Promise<AtheneAiAdminResponse> {
    return this.http
      .post<
        ApiEnvelope<AtheneAiAdminResponse>
      >(`/admin/athene-ai/requests/${workspaceId}/reject`, {})
      .then(unwrap);
  }
}
