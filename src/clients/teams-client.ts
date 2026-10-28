import type {
  AddTeamMembersRequest,
  CreateTeamRequest,
  DeleteTeamRequest,
  DeleteTeamResponse,
  TeamListQuery,
  TeamResponse,
  TeamRole,
  UpdateTeamRequest,
} from '../payloads/teams';
import type { AthenosHttpClient } from '../http';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

/**
 * Teams API surface — workspace-scoped groups of members.
 *
 * Every route is membership-checked: a workspace or team the caller cannot
 * see answers 403, whether or not it exists. Every member reads; owners and
 * admins manage; anyone may leave a team (`removeMember` with their own id).
 */
export class TeamsApiClient {
  constructor(private readonly http: AthenosHttpClient) {}

  /** By name. Members are a preview (`TEAM_MEMBER_PREVIEW`); see `member_count`. */
  list(workspaceId: string, query: TeamListQuery = {}): Promise<TeamResponse[]> {
    const params = new URLSearchParams();
    if (query.member) params.set('member', query.member);
    const qs = params.toString();
    return this.http
      .get<ApiEnvelope<TeamResponse[]>>(
        `/work/workspaces/${workspaceId}/teams${qs ? `?${qs}` : ''}`,
      )
      .then(unwrap);
  }

  /** The caller's own teams in the workspace. */
  listMine(workspaceId: string): Promise<TeamResponse[]> {
    return this.list(workspaceId, { member: 'me' });
  }

  /** With every member. */
  get(id: string): Promise<TeamResponse> {
    return this.http.get<ApiEnvelope<TeamResponse>>(`/work/teams/${id}`).then(unwrap);
  }

  create(workspaceId: string, body: CreateTeamRequest): Promise<TeamResponse> {
    return this.http
      .post<ApiEnvelope<TeamResponse>>(`/work/workspaces/${workspaceId}/teams`, body)
      .then(unwrap);
  }

  update(id: string, body: UpdateTeamRequest): Promise<TeamResponse> {
    return this.http
      .patch<ApiEnvelope<TeamResponse>>(`/work/teams/${id}`, body)
      .then(unwrap);
  }

  /** Deletes the team and its memberships; people stay in the workspace. */
  delete(id: string): Promise<void> {
    return this.http.delete(`/work/teams/${id}`).then(() => undefined);
  }

  /**
   * Delete a team that queues target (W4-F): its queues move to
   * `replacement_team_id` in the same statement. A plain `delete` of such a
   * team is `409` with `details.reason` `team_has_queues`.
   */
  deleteWithReplacement(id: string, body: DeleteTeamRequest): Promise<DeleteTeamResponse> {
    return this.http
      .post<ApiEnvelope<DeleteTeamResponse>>(`/work/teams/${id}/delete`, body)
      .then(unwrap);
  }

  /** Returns the team with every member. */
  addMembers(id: string, body: AddTeamMembersRequest): Promise<TeamResponse> {
    return this.http
      .post<ApiEnvelope<TeamResponse>>(`/work/teams/${id}/members`, body)
      .then(unwrap);
  }

  /** Returns the team with every member. */
  updateMemberRole(id: string, userId: string, role: TeamRole): Promise<TeamResponse> {
    return this.http
      .patch<ApiEnvelope<TeamResponse>>(`/work/teams/${id}/members/${userId}`, { role })
      .then(unwrap);
  }

  /** Someone else (owners/admins), or yourself — which is leaving. */
  removeMember(id: string, userId: string): Promise<void> {
    return this.http
      .delete(`/work/teams/${id}/members/${userId}`)
      .then(() => undefined);
  }
}
