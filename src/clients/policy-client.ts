import type { AthenosHttpClient } from '../http';
import type { CapabilitiesResponse } from '../payloads/policy';
import type { PlaceQuery } from '../payloads/overrides';

interface ApiEnvelope<T> {
  data: T;
}

/**
 * `?project_id=…` / `?doc_collection_id=…` / `?team_id=…` (phase B: the
 * verdicts at that place, through its overrides); empty without one.
 */
export function placeQuery(place: PlaceQuery): string {
  const params = new URLSearchParams();
  if (place.project_id) params.set('project_id', place.project_id);
  if (place.doc_collection_id) params.set('doc_collection_id', place.doc_collection_id);
  if (place.team_id) params.set('team_id', place.team_id);
  const qs = params.toString();
  return qs ? `?${qs}` : '';
}

/** The caller's permissions, as the server decides them. */
export class PolicyApiClient {
  constructor(private readonly http: AthenosHttpClient) {}

  /**
   * Every action's verdict in one workspace (`no-store`: ask again after a
   * workspace switch, an own role change, a 403 on a mutation, or when
   * `policy_revision` differs from the one the client holds). 403 for a
   * non-member. With a place (a project or a docs area of the workspace),
   * the verdicts there: its overrides applied (custom roles, phase B).
   */
  capabilities(
    workspaceId: string,
    place: Pick<PlaceQuery, 'project_id' | 'doc_collection_id'> = {},
  ): Promise<CapabilitiesResponse> {
    return this.http
      .get<ApiEnvelope<CapabilitiesResponse>>(
        `/work/workspaces/${workspaceId}/capabilities${placeQuery(place)}`,
      )
      .then((envelope) => envelope.data);
  }
}
