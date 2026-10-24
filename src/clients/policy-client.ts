import type { AtheneHttpClient } from '../http';
import type { CapabilitiesResponse } from '../payloads/policy';

interface ApiEnvelope<T> {
  data: T;
}

/** The caller's permissions, as the server decides them. */
export class PolicyApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  /**
   * Every action's verdict in one workspace (`no-store`: ask again after a
   * workspace switch, an own role change, a 403 on a mutation, or when
   * `policy_revision` differs from the one the client holds). 403 for a
   * non-member.
   */
  capabilities(workspaceId: string): Promise<CapabilitiesResponse> {
    return this.http
      .get<ApiEnvelope<CapabilitiesResponse>>(`/work/workspaces/${workspaceId}/capabilities`)
      .then((envelope) => envelope.data);
  }
}
