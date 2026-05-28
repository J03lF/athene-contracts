import type {
  AppliesTo,
  CreateFieldDefinitionRequest,
  FieldDefinitionResponse,
  ReorderFieldDefinitionsRequest,
  UpdateFieldDefinitionRequest,
} from '../payloads/field-definitions';
import type { AtheneHttpClient } from '../http';

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
 * `workspace_id` after creation; updates may only change `label`,
 * `options`, `required`, and `position`.
 */
export class FieldDefinitionsApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  /** All definitions for a workspace, ordered by (applies_to, position, key). */
  listForWorkspace(workspaceId: string): Promise<FieldDefinitionResponse[]> {
    return this.http
      .get<ApiEnvelope<FieldDefinitionResponse[]>>(
        `/work/workspaces/${workspaceId}/field-definitions`,
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

  /** POST-delete (mirrors the labels pattern — no body, no envelope). */
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
