import type {
  AddWatcherRequest,
  AssignTicketsRequest,
  CreateLabelRequest,
  CreateSprintRequest,
  CreateTicketLinkRequest,
  LabelResponse,
  LinkRelation,
  SetLabelsRequest,
  SprintBurnDownResponse,
  SprintResponse,
  TicketLinkResponse,
  WatcherResponse,
} from '../payloads/work';
import type { AtheneHttpClient } from '../http';

export class WorkApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  // ---------- Sprints ----------
  listSprintsForProject(projectId: string): Promise<SprintResponse[]> {
    return this.http.get(`/work/projects/${projectId}/sprints`);
  }
  currentSprint(projectId: string): Promise<SprintResponse | null> {
    return this.http.get(`/work/projects/${projectId}/sprints/current`);
  }
  createSprint(projectId: string, body: CreateSprintRequest): Promise<SprintResponse> {
    return this.http.post(`/work/projects/${projectId}/sprints`, body);
  }
  getSprint(id: string): Promise<SprintResponse> {
    return this.http.get(`/work/sprints/${id}`);
  }
  startSprint(id: string): Promise<SprintResponse> {
    return this.http.post(`/work/sprints/${id}/start`);
  }
  completeSprint(id: string): Promise<SprintResponse> {
    return this.http.post(`/work/sprints/${id}/complete`);
  }
  deleteSprint(id: string): Promise<void> {
    return this.http.post(`/work/sprints/${id}/delete`);
  }
  assignTicketsToSprint(id: string, body: AssignTicketsRequest): Promise<void> {
    return this.http.post(`/work/sprints/${id}/assign`, body);
  }
  sprintBurnDown(id: string): Promise<SprintBurnDownResponse> {
    return this.http.get(`/work/sprints/${id}/burndown`);
  }

  // ---------- Labels ----------
  listLabelsForWorkspace(workspaceId: string): Promise<LabelResponse[]> {
    return this.http.get(`/work/workspaces/${workspaceId}/labels`);
  }
  createLabel(workspaceId: string, body: CreateLabelRequest): Promise<LabelResponse> {
    return this.http.post(`/work/workspaces/${workspaceId}/labels`, body);
  }
  listLabelsForTicket(ticketId: string): Promise<LabelResponse[]> {
    return this.http.get(`/work/tickets/${ticketId}/labels`);
  }
  setLabelsForTicket(ticketId: string, body: SetLabelsRequest): Promise<void> {
    return this.http.post(`/work/tickets/${ticketId}/labels/set`, body);
  }
  deleteLabel(id: string): Promise<void> {
    return this.http.post(`/work/labels/${id}/delete`);
  }

  // ---------- Ticket links (V1 client surface; endpoint comes in 3.6) ----------
  listTicketLinks(ticketId: string): Promise<TicketLinkResponse[]> {
    return this.http.get(`/work/tickets/${ticketId}/links`);
  }
  createTicketLink(ticketId: string, body: CreateTicketLinkRequest): Promise<void> {
    return this.http.post(`/work/tickets/${ticketId}/links`, body);
  }
  deleteTicketLink(ticketId: string, toTicketId: string, relation: LinkRelation): Promise<void> {
    return this.http.post(
      `/work/tickets/${ticketId}/links/delete?to=${encodeURIComponent(toTicketId)}&relation=${encodeURIComponent(relation)}`,
    );
  }

  // ---------- Watchers ----------
  listWatchers(ticketId: string): Promise<WatcherResponse[]> {
    return this.http.get(`/work/tickets/${ticketId}/watchers`);
  }
  addWatcher(ticketId: string, body: AddWatcherRequest): Promise<WatcherResponse> {
    return this.http.post(`/work/tickets/${ticketId}/watchers`, body);
  }
  removeWatcher(ticketId: string, userId: string): Promise<void> {
    return this.http.post(`/work/tickets/${ticketId}/watchers/${userId}/remove`);
  }
}
