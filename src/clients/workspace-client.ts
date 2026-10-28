import type { AthenosHttpClient } from '../http';
import type {
  CreateInviteRequest,
  InvitationResponse,
  RenameWorkspaceRequest,
  UpdateWorkspaceRequest,
  UpdateMemberRoleRequest,
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
  constructor(private readonly http: AthenosHttpClient) {}

  listWorkspaces(): Promise<WorkspaceResponse[]> {
    return this.http.get<ApiEnvelope<WorkspaceResponse[]>>('/workspaces').then(unwrap);
  }

  /** Rename a workspace. Its owners and admins only; the slug stays. */
  renameWorkspace(workspaceId: string, body: RenameWorkspaceRequest): Promise<WorkspaceResponse> {
    return this.http
      .patch<ApiEnvelope<WorkspaceResponse>>(`/workspaces/${workspaceId}`, body)
      .then(unwrap);
  }

  /**
   * Change name, description, colour and/or settings (settings merged key
   * by key). Owners and admins only.
   */
  updateWorkspace(workspaceId: string, body: UpdateWorkspaceRequest): Promise<WorkspaceResponse> {
    return this.http
      .patch<ApiEnvelope<WorkspaceResponse>>(`/workspaces/${workspaceId}`, body)
      .then(unwrap);
  }

  /** Members of one workspace, with their identities resolved. */
  listMembers(workspaceId: string): Promise<WorkspaceMemberResponse[]> {
    return this.http
      .get<ApiEnvelope<WorkspaceMemberResponse[]>>(`/workspaces/${workspaceId}/members`)
      .then(unwrap);
  }

  /** Invitations issued for a workspace, newest first, without tokens. Owners and admins only. */
  listInvites(workspaceId: string): Promise<InvitationResponse[]> {
    return this.http
      .get<ApiEnvelope<InvitationResponse[]>>(`/workspaces/${workspaceId}/invites`)
      .then(unwrap);
  }

  /**
   * Mint an invitation. Owners and admins only. The answer carries the
   * link's `token` — the only time it is shown; an open link grants at
   * most `member`.
   */
  createInvite(
    workspaceId: string,
    body: CreateInviteRequest = {},
  ): Promise<InvitationResponse> {
    return this.http
      .post<ApiEnvelope<InvitationResponse>>(`/workspaces/${workspaceId}/invites`, body)
      .then(unwrap);
  }

  /**
   * Change a member's role. Owners and admins only; only an owner touches
   * owners; 409 when it would leave the workspace without an owner.
   */
  updateMemberRole(
    workspaceId: string,
    userId: string,
    body: UpdateMemberRoleRequest,
  ): Promise<WorkspaceMemberResponse> {
    return this.http
      .patch<ApiEnvelope<WorkspaceMemberResponse>>(
        `/workspaces/${workspaceId}/members/${userId}`,
        body,
      )
      .then(unwrap);
  }

  /** Remove a member (owners and admins; only an owner removes an owner). */
  removeMember(workspaceId: string, userId: string): Promise<void> {
    return this.http
      .delete(`/workspaces/${workspaceId}/members/${userId}`)
      .then(() => undefined);
  }

  /** Leave a workspace. 409 for its last owner. Access ends immediately. */
  leaveWorkspace(workspaceId: string, ownUserId: string): Promise<void> {
    return this.removeMember(workspaceId, ownUserId);
  }

  /**
   * Give a live invitation a new link: the old one stops working at once,
   * and the answer carries the new `token` (shown this once). 400 for an
   * invite that is no longer active.
   */
  rotateInvite(workspaceId: string, inviteId: string): Promise<InvitationResponse> {
    return this.http
      .post<ApiEnvelope<InvitationResponse>>(
        `/workspaces/${workspaceId}/invites/${inviteId}/rotate`,
        {},
      )
      .then(unwrap);
  }

  /** Revoke an invite so its link stops working. Idempotent. */
  revokeInvite(workspaceId: string, inviteId: string): Promise<InvitationResponse> {
    return this.http
      .post<ApiEnvelope<InvitationResponse>>(
        `/workspaces/${workspaceId}/invites/${inviteId}/revoke`,
        {},
      )
      .then(unwrap);
  }
}
