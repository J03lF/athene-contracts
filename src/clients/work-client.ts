import type {
  AssignTicketRequest,
  BulkUpdateTicketsRequest,
  BulkUpdateTicketsResponse,
  AssignTicketsRequest,
  AddWorkLogRequest,
  CompleteSprintRequest,
  UpdateSprintRequest,
  UpdateSprintResponse,
  CreateMilestoneRequest,
  MilestoneResponse,
  UpdateMilestoneRequest,
  WorkLogListResponse,
  WorkLogResponse,
  ProjectReportResponse,
  DeletedTicketResponse,
  CreateLabelRequest,
  LabelAssignmentResponse,
  CreateProjectRequest,
  CreateSprintRequest,
  CreateTicketLinkRequest,
  CreateTicketRequest,
  InboxEntryResponse,
  LabelResponse,
  LinkRelation,
  ListProjectsQuery,
  PerUserListQuery,
  ListTicketsQuery,
  ProjectResponse,
  SetLabelsRequest,
  UpdateLabelRequest,
  WorkActivityEntryResponse,
  SprintBurnDownResponse,
  SprintResponse,
  TicketLinkResponse,
  TicketResponse,
  TriageTicketRequest,
  TicketWatcherListResponse,
  UpdateCustomValuesRequest,
  UpdateProjectRequest,
  UpdateTicketRequest,
  AddTicketCommentRequest,
  MoveTicketRequest,
  TicketCommentResponse,
  UpdateTicketStatusRequest,
  WatchStateResponse,
  WorkStatsResponse,
} from '../payloads/work';
import type { AtheneHttpClient, RequestOptions } from '../http';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

/** `?workspace_id=…`, or nothing without a workspace. */
function workspaceQs(workspaceId: string | null | undefined): string {
  return workspaceId ? `?workspace_id=${encodeURIComponent(workspaceId)}` : '';
}

function perUserQs(q: PerUserListQuery): string {
  const params = new URLSearchParams();
  if (q.limit != null) params.set('limit', String(q.limit));
  if (q.workspace_id) params.set('workspace_id', q.workspace_id);
  const qs = params.toString();
  return qs ? `?${qs}` : '';
}

export class WorkApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  // ---------- Projects ----------
  /** Without `workspace_id`: every workspace the caller belongs to. */
  listProjects(query: ListProjectsQuery = {}): Promise<ProjectResponse[]> {
    const qs = query.workspace_id
      ? `?workspace_id=${encodeURIComponent(query.workspace_id)}`
      : '';
    return this.http
      .get<ApiEnvelope<ProjectResponse[]>>(`/work/projects${qs}`)
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
  /**
   * `inbox(20)` still works; `inbox({ limit, workspace_id })` keeps it to
   * one workspace (403 for a non-member).
   */
  inbox(arg?: number | PerUserListQuery): Promise<InboxEntryResponse[]> {
    const q: PerUserListQuery = typeof arg === 'number' ? { limit: arg } : (arg ?? {});
    return this.http
      .get<ApiEnvelope<InboxEntryResponse[]>>(`/work/inbox${perUserQs(q)}`)
      .then(unwrap);
  }

  /**
   * Recent work changes from the audit trail, newest first: only projects
   * and tickets the caller can open; one workspace's with `workspaceId`
   * (403 for a non-member), else every workspace the caller is in.
   * `activity({ limit, workspace_id })` still works.
   */
  activity(
    workspaceId?: string | null,
    limit?: number,
    init?: RequestOptions,
  ): Promise<WorkActivityEntryResponse[]>;
  activity(query: PerUserListQuery): Promise<WorkActivityEntryResponse[]>;
  activity(
    arg?: string | null | PerUserListQuery,
    limit?: number,
    init: RequestOptions = {},
  ): Promise<WorkActivityEntryResponse[]> {
    const q: PerUserListQuery =
      arg !== null && typeof arg === 'object'
        ? arg
        : { ...(arg ? { workspace_id: arg } : {}), ...(limit != null ? { limit } : {}) };
    return this.http
      .get<ApiEnvelope<WorkActivityEntryResponse[]>>(`/work/activity${perUserQs(q)}`, {
        signal: init.signal,
      })
      .then(unwrap);
  }

  /**
   * Open and in-progress tickets and the project count, for the dashboard:
   * one workspace's with `workspaceId` (403 for a non-member), else every
   * workspace the caller is in.
   */
  stats(workspaceId?: string | null, init: RequestOptions = {}): Promise<WorkStatsResponse> {
    return this.http
      .get<ApiEnvelope<WorkStatsResponse>>(`/work/stats${workspaceQs(workspaceId)}`, {
        signal: init.signal,
      })
      .then(unwrap);
  }

  /** The caller's assigned tickets, as full tickets; scoped like `stats`. */
  myTickets(workspaceId?: string | null, init: RequestOptions = {}): Promise<TicketResponse[]> {
    return this.http
      .get<ApiEnvelope<TicketResponse[]>>(`/work/my-tickets${workspaceQs(workspaceId)}`, {
        signal: init.signal,
      })
      .then(unwrap);
  }

  // ---------- Tickets ----------
  listTickets(query: ListTicketsQuery = {}): Promise<TicketResponse[]> {
    const params = new URLSearchParams();
    if (query.project_id) params.set('project_id', query.project_id);
    if (query.workspace_id) params.set('workspace_id', query.workspace_id);
    const qs = params.toString() ? `?${params.toString()}` : '';
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
  /**
   * Delete a ticket for good, with its subtasks, comments, labels, links
   * and watchers. Operators only.
   */
  deleteTicket(id: string): Promise<DeletedTicketResponse> {
    return this.http
      .delete<ApiEnvelope<DeletedTicketResponse>>(`/work/tickets/${id}`)
      .then(unwrap);
  }
  /**
   * Apply one patch to many tickets. Answers per ticket — check `failed`.
   */
  updateTickets(
    body: BulkUpdateTicketsRequest,
  ): Promise<BulkUpdateTicketsResponse> {
    return this.http
      .post<ApiEnvelope<BulkUpdateTicketsResponse>>('/work/tickets/bulk', body)
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
  /** Change a comment's text. Its author only. */
  editTicketComment(
    ticketId: string,
    commentId: string,
    body: AddTicketCommentRequest,
  ): Promise<TicketCommentResponse> {
    return this.http
      .patch<ApiEnvelope<TicketCommentResponse>>(
        `/work/tickets/${ticketId}/comments/${commentId}`,
        body,
      )
      .then(unwrap);
  }
  /** Remove a comment. Its author, or an operator. */
  deleteTicketComment(ticketId: string, commentId: string): Promise<void> {
    return this.http
      .delete(`/work/tickets/${ticketId}/comments/${commentId}`)
      .then(() => undefined);
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
  /** Any workspace member but a guest. */
  triageTicket(id: string, body: TriageTicketRequest): Promise<TicketResponse> {
    return this.http
      .patch<ApiEnvelope<TicketResponse>>(`/work/tickets/${id}/triage`, body)
      .then(unwrap);
  }
  updateTicketCustomValues(
    id: string,
    body: UpdateCustomValuesRequest,
  ): Promise<TicketResponse> {
    return this.http
      .patch<ApiEnvelope<TicketResponse>>(
        `/work/tickets/${id}/custom-values`,
        body,
      )
      .then(unwrap);
  }
  updateProject(id: string, body: UpdateProjectRequest): Promise<ProjectResponse> {
    return this.http
      .patch<ApiEnvelope<ProjectResponse>>(`/work/projects/${id}`, body)
      .then(unwrap);
  }
  updateProjectCustomValues(
    id: string,
    body: UpdateCustomValuesRequest,
  ): Promise<ProjectResponse> {
    return this.http
      .patch<ApiEnvelope<ProjectResponse>>(
        `/work/projects/${id}/custom-values`,
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
  /** Move a sprint's dates, optionally with the own due dates named. */
  updateSprint(id: string, body: UpdateSprintRequest): Promise<UpdateSprintResponse> {
    return this.http
      .post<ApiEnvelope<SprintResponse> & { moved_tickets?: number }>(
        `/work/sprints/${id}/update`,
        body
      )
      .then((res) => ({ sprint: unwrap(res), moved_tickets: res.moved_tickets ?? 0 }));
  }
  /** Complete a sprint; its unfinished tickets move as `body` says. */
  completeSprint(id: string, body: CompleteSprintRequest = {}): Promise<SprintResponse> {
    return this.http
      .post<ApiEnvelope<SprintResponse>>(`/work/sprints/${id}/complete`, body)
      .then(unwrap);
  }
  deleteSprint(id: string): Promise<void> {
    return this.http.post(`/work/sprints/${id}/delete`).then(() => undefined);
  }
  assignTicketsToSprint(id: string, body: AssignTicketsRequest): Promise<void> {
    return this.http.post(`/work/sprints/${id}/assign`, body).then(() => undefined);
  }
  /** Take tickets back out of a sprint, into the backlog. */
  unassignTicketsFromSprint(id: string, body: AssignTicketsRequest): Promise<void> {
    return this.http.post(`/work/sprints/${id}/unassign`, body).then(() => undefined);
  }
  // ---------- Work logs ----------
  listWorkLogs(ticketId: string): Promise<WorkLogListResponse> {
    return this.http
      .get<ApiEnvelope<WorkLogListResponse>>(`/work/tickets/${ticketId}/worklogs`)
      .then(unwrap);
  }
  addWorkLog(ticketId: string, body: AddWorkLogRequest): Promise<WorkLogResponse> {
    return this.http
      .post<ApiEnvelope<WorkLogResponse>>(`/work/tickets/${ticketId}/worklogs`, body)
      .then(unwrap);
  }
  deleteWorkLog(id: string): Promise<void> {
    return this.http.delete(`/work/worklogs/${id}`).then(() => undefined);
  }

  // ---------- Releases ----------
  listMilestones(projectId: string): Promise<MilestoneResponse[]> {
    return this.http
      .get<ApiEnvelope<MilestoneResponse[]>>(`/work/projects/${projectId}/milestones`)
      .then(unwrap);
  }
  createMilestone(projectId: string, body: CreateMilestoneRequest): Promise<MilestoneResponse> {
    return this.http
      .post<ApiEnvelope<MilestoneResponse>>(`/work/projects/${projectId}/milestones`, body)
      .then(unwrap);
  }
  updateMilestone(id: string, body: UpdateMilestoneRequest): Promise<MilestoneResponse> {
    return this.http
      .patch<ApiEnvelope<MilestoneResponse>>(`/work/milestones/${id}`, body)
      .then(unwrap);
  }
  deleteMilestone(id: string): Promise<void> {
    return this.http.delete(`/work/milestones/${id}`).then(() => undefined);
  }

  /**
   * Burndown, velocity and flow for a project. `sprintId` picks the sprint
   * whose burndown is drawn; by default the running one, else the last
   * completed.
   */
  projectReport(projectId: string, sprintId?: string): Promise<ProjectReportResponse> {
    const query = sprintId ? `?sprint_id=${encodeURIComponent(sprintId)}` : '';
    return this.http
      .get<ApiEnvelope<ProjectReportResponse>>(`/work/projects/${projectId}/reports${query}`)
      .then(unwrap);
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
  updateLabel(id: string, body: UpdateLabelRequest): Promise<LabelResponse> {
    return this.http.patch(`/work/labels/${id}`, body);
  }
  deleteLabel(id: string): Promise<void> {
    return this.http.post(`/work/labels/${id}/delete`);
  }
  /**
   * Every (ticket, label) pair in a workspace — one read for the chips and
   * the filter of a whole list or board.
   */
  listLabelAssignments(workspaceId: string): Promise<LabelAssignmentResponse[]> {
    return this.http.get(`/work/workspaces/${workspaceId}/labels/assignments`);
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
