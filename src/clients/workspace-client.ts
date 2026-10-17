import type { AtheneHttpClient } from '../http';
import type {
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
}
