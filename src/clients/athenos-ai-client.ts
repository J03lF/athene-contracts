import type { AthenosHttpClient } from '../http';
import type {
  AiModelLicensesResponse,
  AthenosAiAdminResponse,
  AthenosAiWorkspaceResponse,
  EnrollAiWorkspaceRequest,
  UpdateAthenosAiRequest,
} from '../payloads/athenos-ai';

interface ApiEnvelope<T> {
  data: T;
}

const unwrap = <T>(envelope: ApiEnvelope<T>): T => envelope.data;

/**
 * Athenos AI: a workspace's page and beta enrollment, and the platform
 * control in the admin console. Every admin call answers the whole
 * console state.
 */
export class AthenosAiApiClient {
  constructor(private readonly http: AthenosHttpClient) {}

  /** The workspace's Athenos AI page (members). */
  workspace(workspaceId: string): Promise<AthenosAiWorkspaceResponse> {
    return this.http
      .get<ApiEnvelope<AthenosAiWorkspaceResponse>>(`/work/workspaces/${workspaceId}/athenos-ai`)
      .then(unwrap);
  }

  /** Apply for the beta (owners and admins, `preview` only). */
  requestBeta(workspaceId: string): Promise<AthenosAiWorkspaceResponse> {
    return this.http
      .post<
        ApiEnvelope<AthenosAiWorkspaceResponse>
      >(`/work/workspaces/${workspaceId}/athenos-ai/beta`, {})
      .then(unwrap);
  }

  /** Withdraw an open request. */
  withdrawBeta(workspaceId: string): Promise<AthenosAiWorkspaceResponse> {
    return this.http
      .delete<ApiEnvelope<AthenosAiWorkspaceResponse>>(`/work/workspaces/${workspaceId}/athenos-ai/beta`)
      .then(unwrap);
  }

  /** The platform control (platform admins). */
  admin(): Promise<AthenosAiAdminResponse> {
    return this.http.get<ApiEnvelope<AthenosAiAdminResponse>>('/admin/athenos-ai').then(unwrap);
  }

  updateAdmin(body: UpdateAthenosAiRequest): Promise<AthenosAiAdminResponse> {
    return this.http
      .put<ApiEnvelope<AthenosAiAdminResponse>>('/admin/athenos-ai', body)
      .then(unwrap);
  }

  enroll(body: EnrollAiWorkspaceRequest): Promise<AthenosAiAdminResponse> {
    return this.http
      .post<ApiEnvelope<AthenosAiAdminResponse>>('/admin/athenos-ai/workspaces', body)
      .then(unwrap);
  }

  removeWorkspace(workspaceId: string): Promise<AthenosAiAdminResponse> {
    return this.http
      .delete<ApiEnvelope<AthenosAiAdminResponse>>(`/admin/athenos-ai/workspaces/${workspaceId}`)
      .then(unwrap);
  }

  approve(workspaceId: string): Promise<AthenosAiAdminResponse> {
    return this.http
      .post<
        ApiEnvelope<AthenosAiAdminResponse>
      >(`/admin/athenos-ai/requests/${workspaceId}/approve`, {})
      .then(unwrap);
  }

  reject(workspaceId: string): Promise<AthenosAiAdminResponse> {
    return this.http
      .post<
        ApiEnvelope<AthenosAiAdminResponse>
      >(`/admin/athenos-ai/requests/${workspaceId}/reject`, {})
      .then(unwrap);
  }

  /** The licence files shipped with the quality model (admins). */
  modelLicenses(version?: string): Promise<AiModelLicensesResponse> {
    const query = version ? `?version=${encodeURIComponent(version)}` : '';
    return this.http
      .get<ApiEnvelope<AiModelLicensesResponse>>(`/admin/athenos-ai/model/licenses${query}`)
      .then(unwrap);
  }
}
