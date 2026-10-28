import type { AthenosHttpClient } from '../http';
import type {
  CreateIntegrationRequest,
  DeleteIntegrationResponse,
  DeliveryQueuedResponse,
  IntegrationCreatedResponse,
  IntegrationDeliveriesResponse,
  IntegrationResponse,
  IntegrationSecretResponse,
  UpdateIntegrationRequest,
  WorkspaceIntegrationsResponse,
} from '../payloads/integrations';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

/**
 * A workspace's integrations (W13-W): webhooks, Slack and Teams. Every call
 * is `workspace.settings.edit` (owners and admins). Nothing is sent inside a
 * request: a test or a redelivery is queued (`202`) and the worker sends it
 * within seconds; read the deliveries again for the outcome.
 */
export class IntegrationsApiClient {
  constructor(private readonly http: AthenosHttpClient) {}

  private base(workspaceId: string): string {
    return `/work/workspaces/${workspaceId}/integrations`;
  }

  list(workspaceId: string): Promise<WorkspaceIntegrationsResponse> {
    return this.http
      .get<ApiEnvelope<WorkspaceIntegrationsResponse>>(this.base(workspaceId))
      .then(unwrap);
  }

  /** A webhook's `signing_secret` is in this answer only. */
  create(workspaceId: string, body: CreateIntegrationRequest): Promise<IntegrationCreatedResponse> {
    return this.http
      .post<ApiEnvelope<IntegrationCreatedResponse>>(this.base(workspaceId), body)
      .then(unwrap);
  }

  /** `409 ATH-RES-003` with `details.current` when `expected_version` moved. */
  update(
    workspaceId: string,
    integrationId: string,
    body: UpdateIntegrationRequest,
  ): Promise<IntegrationResponse> {
    return this.http
      .patch<ApiEnvelope<IntegrationResponse>>(`${this.base(workspaceId)}/${integrationId}`, body)
      .then(unwrap);
  }

  remove(workspaceId: string, integrationId: string): Promise<DeleteIntegrationResponse> {
    return this.http
      .delete<ApiEnvelope<DeleteIntegrationResponse>>(`${this.base(workspaceId)}/${integrationId}`)
      .then(unwrap);
  }

  /** A new signing secret (webhooks), shown once; the old one stops at once. */
  rotateSecret(workspaceId: string, integrationId: string): Promise<IntegrationSecretResponse> {
    return this.http
      .post<ApiEnvelope<IntegrationSecretResponse>>(
        `${this.base(workspaceId)}/${integrationId}/secret`,
        {},
      )
      .then(unwrap);
  }

  /** "Test senden": `429 ATH-RATE-001` past the per-minute budget. */
  test(workspaceId: string, integrationId: string): Promise<DeliveryQueuedResponse> {
    return this.http
      .post<ApiEnvelope<DeliveryQueuedResponse>>(
        `${this.base(workspaceId)}/${integrationId}/test`,
        {},
      )
      .then(unwrap);
  }

  /** The last deliveries, newest first. */
  deliveries(workspaceId: string, integrationId: string): Promise<IntegrationDeliveriesResponse> {
    return this.http
      .get<ApiEnvelope<IntegrationDeliveriesResponse>>(
        `${this.base(workspaceId)}/${integrationId}/deliveries`,
      )
      .then(unwrap);
  }

  /** The same body again, with a new delivery id and a fresh signature. */
  redeliver(
    workspaceId: string,
    integrationId: string,
    deliveryId: string,
  ): Promise<DeliveryQueuedResponse> {
    return this.http
      .post<ApiEnvelope<DeliveryQueuedResponse>>(
        `${this.base(workspaceId)}/${integrationId}/deliveries/${deliveryId}/redeliver`,
        {},
      )
      .then(unwrap);
  }
}
