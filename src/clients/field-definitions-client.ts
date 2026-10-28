import type {
  AppliesTo,
  CreateFieldDefinitionRequest,
  FieldDefinitionResponse,
  ListFieldDefinitionsQuery,
  ReorderFieldDefinitionsRequest,
  UpdateFieldDefinitionRequest,
} from '../payloads/field-definitions';
import type { AthenosHttpClient } from '../http';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

/**
 * Field-definitions API surface — workspace-scoped custom-field schema.
 *
 * Definitions decorate either tickets or projects (`AppliesTo`). The server
 * enforces immutability of `key`, `field_type`, `applies_to`, and
 * `workspace_id` after creation. Where a ticket field applies is its rules
 * (`FieldAssignmentsApiClient`). Archiving replaces deleting: a definition
 * is deleted only from the archive.
 */
export class FieldDefinitionsApiClient {
  constructor(private readonly http: AthenosHttpClient) {}

  /**
   * All definitions for a workspace, ordered by (applies_to, position, key);
   * archived ones only with `include_archived`, value counts only with
   * `with_usage` (for managers).
   */
  listForWorkspace(
    workspaceId: string,
    query: ListFieldDefinitionsQuery = {},
  ): Promise<FieldDefinitionResponse[]> {
    const params = new URLSearchParams();
    if (query.include_archived) params.set('include_archived', 'true');
    if (query.with_usage) params.set('with_usage', 'true');
    const qs = params.toString();
    return this.http
      .get<ApiEnvelope<FieldDefinitionResponse[]>>(
        `/work/workspaces/${workspaceId}/field-definitions${qs ? `?${qs}` : ''}`,
      )
      .then(unwrap);
  }

  /** Definitions for one scope within a workspace — e.g. all ticket fields. */
  listForScope(
    workspaceId: string,
    scope: AppliesTo,
  ): Promise<FieldDefinitionResponse[]> {
    return this.http
      .get<ApiEnvelope<FieldDefinitionResponse[]>>(
        `/work/workspaces/${workspaceId}/field-definitions/scope/${scope}`,
      )
      .then(unwrap);
  }

  create(
    workspaceId: string,
    body: CreateFieldDefinitionRequest,
  ): Promise<FieldDefinitionResponse> {
    return this.http
      .post<ApiEnvelope<FieldDefinitionResponse>>(
        `/work/workspaces/${workspaceId}/field-definitions`,
        body,
      )
      .then(unwrap);
  }

  update(
    id: string,
    body: UpdateFieldDefinitionRequest,
  ): Promise<FieldDefinitionResponse> {
    return this.http
      .patch<ApiEnvelope<FieldDefinitionResponse>>(
        `/work/field-definitions/${id}`,
        body,
      )
      .then(unwrap);
  }

  /** Not offered anywhere new; its rules rest, its values stay. */
  archive(id: string): Promise<FieldDefinitionResponse> {
    return this.http
      .post<ApiEnvelope<FieldDefinitionResponse>>(`/work/field-definitions/${id}/archive`, {})
      .then(unwrap);
  }

  restore(id: string): Promise<FieldDefinitionResponse> {
    return this.http
      .post<ApiEnvelope<FieldDefinitionResponse>>(`/work/field-definitions/${id}/restore`, {})
      .then(unwrap);
  }

  /**
   * POST-delete (mirrors the labels pattern — no body, no envelope). Only an
   * archived definition (else `409`); its rules, ticket attachments,
   * suggestions and values go with it.
   */
  delete(id: string): Promise<void> {
    return this.http
      .post<void>(`/work/field-definitions/${id}/delete`, {})
      .then(() => undefined);
  }

  reorder(
    workspaceId: string,
    body: ReorderFieldDefinitionsRequest,
  ): Promise<FieldDefinitionResponse[]> {
    return this.http
      .post<ApiEnvelope<FieldDefinitionResponse[]>>(
        `/work/workspaces/${workspaceId}/field-definitions/reorder`,
        body,
      )
      .then(unwrap);
  }
}
