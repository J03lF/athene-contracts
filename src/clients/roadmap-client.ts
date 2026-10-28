import type { RoadmapQuery, RoadmapResponse } from '../payloads/roadmap';
import type { AthenosHttpClient, RequestOptions } from '../http';

interface ApiEnvelope<T> {
  data: T;
}

/**
 * The roadmap of one workspace (W7-R): its planning items with their
 * dates and subtasks' progress, their `blocks` links, the sprints and the
 * dated releases, in one read. `project_id` narrows it to one project,
 * `parent_id` answers the direct subtasks of a ticket instead.
 *
 * Members read (guests too); a non-member gets 403. Moving a bar is the
 * ticket edit of `WorkApiClient.updateTicket`.
 */
export class RoadmapApiClient {
  constructor(private readonly http: AthenosHttpClient) {}

  roadmap(
    workspaceId: string,
    query: RoadmapQuery = {},
    options: RequestOptions = {}
  ): Promise<RoadmapResponse> {
    const params = new URLSearchParams();
    if (query.project_id) params.set('project_id', query.project_id);
    if (query.parent_id) params.set('parent_id', query.parent_id);
    const qs = params.toString();
    return this.http
      .get<
        ApiEnvelope<RoadmapResponse>
      >(`/work/workspaces/${workspaceId}/roadmap${qs ? `?${qs}` : ''}`, options)
      .then((envelope) => envelope.data);
  }
}
