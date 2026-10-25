import type { AtheneHttpClient, RequestOptions } from '../http';
import type {
  AskReporterRequest,
  AskReporterResponse,
  BacklogReviewLast,
  BacklogReviewPage,
  BacklogReviewQuery,
  BacklogReviewRunRequest,
  CompareEditsRequest,
  CompareEditsResponse,
  DeleteIntelDataResponse,
  RecordDecisionsRequest,
  RecordDecisionsResponse,
  TicketCheckRequest,
  TicketCheckResponse,
} from '../payloads/ticket-check';

interface ApiEnvelope<T> {
  data: T;
}

const unwrap = <T>(envelope: ApiEnvelope<T>): T => envelope.data;

/** The ticket check: check a draft, record decisions, forget what was learned. */
export class TicketCheckApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  /** Checks a draft; stores nothing. */
  check(workspaceId: string, body: TicketCheckRequest): Promise<TicketCheckResponse> {
    return this.http
      .post<ApiEnvelope<TicketCheckResponse>>(`/work/workspaces/${workspaceId}/ticket-check`, body)
      .then(unwrap);
  }

  /** A saved ticket before and after an edit (the writing assistance); stores nothing. */
  compare(workspaceId: string, body: CompareEditsRequest): Promise<CompareEditsResponse> {
    return this.http
      .post<
        ApiEnvelope<CompareEditsResponse>
      >(`/work/workspaces/${workspaceId}/ticket-check/compare`, body)
      .then(unwrap);
  }

  /**
   * One page of the backlog review of a project's open tickets: read-only,
   * nothing is written back. `signal` cancels it (the person left).
   */
  reviewBacklog(
    workspaceId: string,
    projectId: string,
    query: BacklogReviewQuery = {},
    options: RequestOptions = {}
  ): Promise<BacklogReviewPage> {
    const params = new URLSearchParams();
    if (query.offset !== undefined) params.set('offset', String(query.offset));
    if (query.limit !== undefined) params.set('limit', String(query.limit));
    if (query.locale !== undefined) params.set('locale', query.locale);
    const qs = params.toString();
    return this.http
      .get<
        ApiEnvelope<BacklogReviewPage>
      >(`/work/workspaces/${workspaceId}/projects/${projectId}/quality-review${qs ? `?${qs}` : ''}`, options)
      .then(unwrap);
  }

  /**
   * One page of a remembered backlog review: the first page (no `run_id`)
   * starts a run, the next ones name the page's `run_id`. Stores ids,
   * versions, scores and criterion states, no ticket text.
   */
  reviewBacklogRun(
    workspaceId: string,
    projectId: string,
    body: BacklogReviewRunRequest,
    options: RequestOptions = {}
  ): Promise<BacklogReviewPage> {
    return this.http
      .post<
        ApiEnvelope<BacklogReviewPage>
      >(`/work/workspaces/${workspaceId}/projects/${projectId}/quality-review/runs`, body, options)
      .then(unwrap);
  }

  /** The project's last finished review, with the trend against the run before. */
  lastBacklogReview(
    workspaceId: string,
    projectId: string,
    locale?: 'de' | 'en',
    options: RequestOptions = {}
  ): Promise<BacklogReviewLast> {
    const qs = locale ? `?locale=${locale}` : '';
    return this.http
      .get<
        ApiEnvelope<BacklogReviewLast>
      >(`/work/workspaces/${workspaceId}/projects/${projectId}/quality-review/last${qs}`, options)
      .then(unwrap);
  }

  /**
   * "Reporter um Ergänzung bitten": posts the confirmed comments as the
   * current person (at most 20; `comment.create` on each ticket).
   */
  askReporters(
    workspaceId: string,
    projectId: string,
    body: AskReporterRequest
  ): Promise<AskReporterResponse> {
    return this.http
      .post<
        ApiEnvelope<AskReporterResponse>
      >(`/work/workspaces/${workspaceId}/projects/${projectId}/quality-review/requests`, body)
      .then(unwrap);
  }

  /** What the person did with suggestions (signed tokens, at most 50). */
  recordDecisions(
    workspaceId: string,
    body: RecordDecisionsRequest
  ): Promise<RecordDecisionsResponse> {
    return this.http
      .post<
        ApiEnvelope<RecordDecisionsResponse>
      >(`/work/workspaces/${workspaceId}/intel/decisions`, body)
      .then(unwrap);
  }

  /** Deletes what the workspace learned (owners and admins). */
  deleteData(workspaceId: string): Promise<DeleteIntelDataResponse> {
    return this.http
      .post<ApiEnvelope<DeleteIntelDataResponse>>(`/work/workspaces/${workspaceId}/intel/data/delete`, {})
      .then(unwrap);
  }
}
