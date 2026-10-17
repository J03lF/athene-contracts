import type {
  AssignTicketRequest,
  AssignTicketsRequest,
  CreateLabelRequest,
  CreateProjectRequest,
  CreateSprintRequest,
  CreateTicketLinkRequest,
  CreateTicketRequest,
  InboxEntryResponse,
  LabelResponse,
  LinkRelation,
  ListTicketsQuery,
  ProjectResponse,
  SetLabelsRequest,
  SprintBurnDownResponse,
  SprintResponse,
  TicketLinkResponse,
  TicketResponse,
  TicketWatcherListResponse,
  UpdateTicketCustomValuesRequest,
  UpdateTicketRequest,
  AddTicketCommentRequest,
  MoveTicketRequest,
  TicketCommentResponse,
  UpdateTicketStatusRequest,
  WatchStateResponse,
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

  // ---------- Inbox ----------
  /**
   * What has happened on the tickets the caller is involved in, newest
   * first. Relevance is decided by the server: it needs the watcher table
   * and ticket ownership, neither of which the client has.
   */
  inbox(limit?: number): Promise<InboxEntryResponse[]> {
    const qs = limit != null ? `?limit=${limit}` : '';
    return this.http
      .get<ApiEnvelope<InboxEntryResponse[]>>(`/work/inbox${qs}`)
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
  /** One ticket by id — for deep links that open before the list loads. */
  getTicket(id: string): Promise<TicketResponse> {
    return this.http
      .get<ApiEnvelope<TicketResponse>>(`/work/tickets/${id}`)
      .then(unwrap);
  }
  /**
   * Content patch. Omitted keys are left alone; an explicit `null` clears a
   * nullable field. See `UpdateTicketRequest`.
   */
  updateTicket(id: string, body: UpdateTicketRequest): Promise<TicketResponse> {
    return this.http
      .patch<ApiEnvelope<TicketResponse>>(`/work/tickets/${id}`, body)
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
  /** Comments on a ticket, oldest first. */
  listTicketComments(id: string): Promise<TicketCommentResponse[]> {
    return this.http
      .get<ApiEnvelope<TicketCommentResponse[]>>(`/work/tickets/${id}/comments`)
      .then(unwrap);
  }

  addTicketComment(
    id: string,
    body: AddTicketCommentRequest,
  ): Promise<TicketCommentResponse> {
    return this.http
      .post<ApiEnvelope<TicketCommentResponse>>(`/work/tickets/${id}/comments`, body)
      .then(unwrap);
  }

  /** Board drag: target column and position in one request. */
  moveTicket(id: string, body: MoveTicketRequest): Promise<TicketResponse> {
    return this.http
      .patch<ApiEnvelope<TicketResponse>>(`/work/tickets/${id}/move`, body)
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
    return this.http
      .get<ApiEnvelope<SprintResponse[]>>(`/work/projects/${projectId}/sprints`)
      .then(unwrap);
  }
  /** The sprint running today, or `null` when none is. */
  currentSprint(projectId: string): Promise<SprintResponse | null> {
    return this.http
      .get<ApiEnvelope<SprintResponse | null>>(`/work/projects/${projectId}/sprints/current`)
      .then(unwrap);
  }
  createSprint(projectId: string, body: CreateSprintRequest): Promise<SprintResponse> {
    return this.http
      .post<ApiEnvelope<SprintResponse>>(`/work/projects/${projectId}/sprints`, body)
      .then(unwrap);
  }
  getSprint(id: string): Promise<SprintResponse> {
    return this.http.get<ApiEnvelope<SprintResponse>>(`/work/sprints/${id}`).then(unwrap);
  }
  startSprint(id: string): Promise<SprintResponse> {
    return this.http
      .post<ApiEnvelope<SprintResponse>>(`/work/sprints/${id}/start`)
      .then(unwrap);
  }
  completeSprint(id: string): Promise<SprintResponse> {
    return this.http
      .post<ApiEnvelope<SprintResponse>>(`/work/sprints/${id}/complete`)
      .then(unwrap);
  }
  deleteSprint(id: string): Promise<void> {
    return this.http.post(`/work/sprints/${id}/delete`).then(() => undefined);
  }
  assignTicketsToSprint(id: string, body: AssignTicketsRequest): Promise<void> {
    return this.http.post(`/work/sprints/${id}/assign`, body).then(() => undefined);
  }
  sprintBurnDown(id: string): Promise<SprintBurnDownResponse> {
    return this.http
      .get<ApiEnvelope<SprintBurnDownResponse>>(`/work/sprints/${id}/burndown`)
      .then(unwrap);
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

  // ---------- Ticket links ----------
  /** Links on a ticket, each carrying the ticket on the other end. */
  listTicketLinks(ticketId: string): Promise<TicketLinkResponse[]> {
    return this.http
      .get<ApiEnvelope<TicketLinkResponse[]>>(`/work/tickets/${ticketId}/links`)
      .then(unwrap);
  }
  createTicketLink(
    ticketId: string,
    body: CreateTicketLinkRequest,
  ): Promise<TicketLinkResponse> {
    return this.http
      .post<ApiEnvelope<TicketLinkResponse>>(`/work/tickets/${ticketId}/links`, body)
      .then(unwrap);
  }
  deleteTicketLink(
    ticketId: string,
    toTicketId: string,
    relation: LinkRelation,
  ): Promise<void> {
    return this.http
      .delete<ApiEnvelope<{ deleted: boolean }>>(
        `/work/tickets/${ticketId}/links/${encodeURIComponent(relation)}/${encodeURIComponent(toTicketId)}`,
      )
      .then(() => undefined);
  }

  // ---------- Watchers ----------
  /** The watcher list plus whether the caller is on it. */
  listWatchers(ticketId: string): Promise<TicketWatcherListResponse> {
    return this.http
      .get<ApiEnvelope<TicketWatcherListResponse>>(`/work/tickets/${ticketId}/watchers`)
      .then(unwrap);
  }
  /**
   * Start watching, as the caller. Watching is self-service — there is no
   * endpoint for subscribing somebody else, by design.
   */
  watchTicket(ticketId: string): Promise<WatchStateResponse> {
    return this.http
      .put<ApiEnvelope<WatchStateResponse>>(`/work/tickets/${ticketId}/watchers/me`)
      .then(unwrap);
  }
  unwatchTicket(ticketId: string): Promise<WatchStateResponse> {
    return this.http
      .delete<ApiEnvelope<WatchStateResponse>>(`/work/tickets/${ticketId}/watchers/me`)
      .then(unwrap);
  }
}
