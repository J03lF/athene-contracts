import type { AtheneHttpClient } from '../http';
import type {
  PlaceQuery,
  SetOverrideRequest,
  SetOverrideResponse,
  WorkspaceOverridesResponse,
} from '../payloads/overrides';
import { placeQuery } from './policy-client';
import type {
  AnswerRolesNoticeRequest,
  CreatePresetRolesRequest,
  CreateRoleRequest,
  DeleteRoleRequest,
  DeleteRoleResponse,
  EffectiveRoleResponse,
  ReorderRolesRequest,
  UpdateRoleRequest,
  WorkspaceRoleResponse,
  WorkspaceRolesResponse,
} from '../payloads/roles';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

/**
 * A workspace's roles (custom roles, phase A). Reading is every member's;
 * every write is `roles.manage` (owners and admins), and each write moves
 * the workspace's `roles_epoch`, so capabilities held elsewhere reload.
 */
export class RolesApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  /** The roles by position, the templates, and an owner's one-time notice. */
  list(workspaceId: string): Promise<WorkspaceRolesResponse> {
    return this.http
      .get<ApiEnvelope<WorkspaceRolesResponse>>(`/workspaces/${workspaceId}/roles`)
      .then(unwrap);
  }

  create(workspaceId: string, body: CreateRoleRequest): Promise<WorkspaceRoleResponse> {
    return this.http
      .post<ApiEnvelope<WorkspaceRoleResponse>>(`/workspaces/${workspaceId}/roles`, body)
      .then(unwrap);
  }

  /** Several templates at once; existing ones are left alone. */
  createPresets(
    workspaceId: string,
    body: CreatePresetRolesRequest,
  ): Promise<WorkspaceRolesResponse> {
    return this.http
      .post<ApiEnvelope<WorkspaceRolesResponse>>(`/workspaces/${workspaceId}/roles/presets`, body)
      .then(unwrap);
  }

  /** 409 with the role as it is now when `version` is stale (3.7). */
  update(
    workspaceId: string,
    roleId: string,
    body: UpdateRoleRequest,
  ): Promise<WorkspaceRoleResponse> {
    return this.http
      .patch<ApiEnvelope<WorkspaceRoleResponse>>(
        `/workspaces/${workspaceId}/roles/${roleId}`,
        body,
      )
      .then(unwrap);
  }

  reorder(workspaceId: string, body: ReorderRolesRequest): Promise<WorkspaceRolesResponse> {
    return this.http
      .post<ApiEnvelope<WorkspaceRolesResponse>>(`/workspaces/${workspaceId}/roles/reorder`, body)
      .then(unwrap);
  }

  /** Its members and open invitations move to the replacement first. */
  delete(workspaceId: string, roleId: string, body: DeleteRoleRequest): Promise<DeleteRoleResponse> {
    return this.http
      .post<ApiEnvelope<DeleteRoleResponse>>(
        `/workspaces/${workspaceId}/roles/${roleId}/delete`,
        body,
      )
      .then(unwrap);
  }

  /**
   * "Vorschau als Rolle": each permission with its verdict, origin and
   * chain — at a place and as a member of a team when named (phase B).
   */
  effective(
    workspaceId: string,
    roleId: string,
    place: PlaceQuery = {},
  ): Promise<EffectiveRoleResponse> {
    return this.http
      .get<ApiEnvelope<EffectiveRoleResponse>>(
        `/workspaces/${workspaceId}/roles/${roleId}/effective${placeQuery(place)}`,
      )
      .then(unwrap);
  }

  /** Every permission override of the workspace (phase B; every member reads them). */
  overrides(workspaceId: string): Promise<WorkspaceOverridesResponse> {
    return this.http
      .get<ApiEnvelope<WorkspaceOverridesResponse>>(`/workspaces/${workspaceId}/overrides`)
      .then(unwrap);
  }

  /**
   * Set one target's override at one place; both lists empty clears it
   * (`roles.manage`). 409 with what is stored now when `version` is stale.
   */
  setOverride(workspaceId: string, body: SetOverrideRequest): Promise<SetOverrideResponse> {
    return this.http
      .put<ApiEnvelope<SetOverrideResponse>>(`/workspaces/${workspaceId}/overrides`, body)
      .then(unwrap);
  }

  /** An owner answers the notice after migration 0032. */
  answerNotice(workspaceId: string, body: AnswerRolesNoticeRequest): Promise<WorkspaceRolesResponse> {
    return this.http
      .post<ApiEnvelope<WorkspaceRolesResponse>>(`/workspaces/${workspaceId}/roles/notice`, body)
      .then(unwrap);
  }
}
