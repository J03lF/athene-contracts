import type { AtheneHttpClient } from '../http';
import type {
  CreateInviteRequest,
  InvitationResponse,
  WorkspaceMemberResponse,
  WorkspaceResponse,
} from '../payloads/workspace';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

/** Workspace and membership endpoints. */
export class WorkspaceApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  listWorkspaces(): Promise<WorkspaceResponse[]> {
    return this.http.get<ApiEnvelope<WorkspaceResponse[]>>('/workspaces').then(unwrap);
  }

  /** Members of one workspace, with their identities resolved. */
  listMembers(workspaceId: string): Promise<WorkspaceMemberResponse[]> {
    return this.http
      .get<ApiEnvelope<WorkspaceMemberResponse[]>>(`/workspaces/${workspaceId}/members`)
      .then(unwrap);
  }

  /** Invitations issued for a workspace, newest first. Members only. */
  listInvites(workspaceId: string): Promise<InvitationResponse[]> {
    return this.http
      .get<ApiEnvelope<InvitationResponse[]>>(`/workspaces/${workspaceId}/invites`)
      .then(unwrap);
  }

  /** Mint an invitation. Owners and admins only. */
  createInvite(
    workspaceId: string,
    body: CreateInviteRequest = {},
  ): Promise<InvitationResponse> {
    return this.http
      .post<ApiEnvelope<InvitationResponse>>(`/workspaces/${workspaceId}/invites`, body)
      .then(unwrap);
  }
}
