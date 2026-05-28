import type {
  AddWatcherRequest,
  AssignTicketRequest,
  AssignTicketsRequest,
  CreateLabelRequest,
  CreateProjectRequest,
  CreateSprintRequest,
  CreateTicketLinkRequest,
  CreateTicketRequest,
  LabelResponse,
  LinkRelation,
  ListTicketsQuery,
  ProjectResponse,
  SetLabelsRequest,
  SprintBurnDownResponse,
  SprintResponse,
  TicketLinkResponse,
  TicketResponse,
  UpdateTicketCustomValuesRequest,
  UpdateTicketStatusRequest,
  WatcherResponse,
} from '../payloads/work';
import type { AtheneHttpClient } from '../http';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

export class WorkApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  // ---------- Projects ----------
  listProjects(): Promise<ProjectResponse[]> {
    return this.http
      .get<ApiEnvelope<ProjectResponse[]>>('/work/projects')
      .then(unwrap);
  }
  createProject(body: CreateProjectRequest): Promise<ProjectResponse> {
    return this.http
      .post<ApiEnvelope<ProjectResponse>>('/work/projects', body)
      .then(unwrap);
  }

  // ---------- Tickets ----------
  listTickets(query: ListTicketsQuery = {}): Promise<TicketResponse[]> {
    const qs = query.project_id
      ? `?project_id=${encodeURIComponent(query.project_id)}`
      : '';
    return this.http
      .get<ApiEnvelope<TicketResponse[]>>(`/work/tickets${qs}`)
      .then(unwrap);
  }
  createTicket(body: CreateTicketRequest): Promise<TicketResponse> {
    return this.http
      .post<ApiEnvelope<TicketResponse>>('/work/tickets', body)
      .then(unwrap);
  }
  updateTicketStatus(
    id: string,
    body: UpdateTicketStatusRequest,
  ): Promise<TicketResponse> {
    return this.http
      .patch<ApiEnvelope<TicketResponse>>(`/work/tickets/${id}/status`, body)
      .then(unwrap);
  }
  assignTicket(
    id: string,
    body: AssignTicketRequest,
  ): Promise<TicketResponse> {
    return this.http
      .patch<ApiEnvelope<TicketResponse>>(`/work/tickets/${id}/assign`, body)
      .then(unwrap);
  }
  updateTicketCustomValues(
    id: string,
    body: UpdateTicketCustomValuesRequest,
  ): Promise<TicketResponse> {
    return this.http
      .patch<ApiEnvelope<TicketResponse>>(
        `/work/tickets/${id}/custom-values`,
        body,
      )
      .then(unwrap);
  }

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
