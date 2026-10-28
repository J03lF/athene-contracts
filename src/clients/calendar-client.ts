import type {
  CalendarResponse,
  CalendarWindowQuery,
  CreateEventRequest,
  EventEdit,
  EventResponse,
  OccurrenceDelete,
  OccurrenceEdit,
} from '../payloads/calendar';
import type { AthenosHttpClient, RequestOptions } from '../http';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

function windowQuery(query: CalendarWindowQuery): string {
  const params = new URLSearchParams({ from: query.from, to: query.to });
  return params.toString();
}

/**
 * The calendar of one workspace (UX9): its events ("Termine") and the
 * sprint and release bands with their open counts.
 *
 * Members read; every member but a guest creates and edits events; the
 * creator or a manager deletes one. A non-member gets 403 for every route,
 * whether or not the id exists. An edit with a stale base answers 409
 * `ATH-RES-003` with the event as it is now in `details.current`.
 */
export class CalendarApiClient {
  constructor(private readonly http: AthenosHttpClient) {}

  /** Events overlapping the window (`from`/`to` are days, both included). */
  listEvents(
    workspaceId: string,
    query: CalendarWindowQuery,
    options: RequestOptions = {}
  ): Promise<EventResponse[]> {
    return this.http
      .get<
        ApiEnvelope<EventResponse[]>
      >(`/work/workspaces/${workspaceId}/events?${windowQuery(query)}`, options)
      .then(unwrap);
  }

  createEvent(workspaceId: string, body: CreateEventRequest): Promise<EventResponse> {
    return this.http
      .post<ApiEnvelope<EventResponse>>(`/work/workspaces/${workspaceId}/events`, body)
      .then(unwrap);
  }

  updateEvent(id: string, body: EventEdit): Promise<EventResponse> {
    return this.http.patch<ApiEnvelope<EventResponse>>(`/work/events/${id}`, body).then(unwrap);
  }

  deleteEvent(id: string): Promise<void> {
    return this.http.delete(`/work/events/${id}`).then(() => undefined);
  }

  /**
   * Edit one occurrence of the series `seriesId` (W6-A): only it, it and
   * every later one, or the whole series (`body.scope`). Answers the
   * occurrence as it is now.
   */
  updateOccurrence(seriesId: string, body: OccurrenceEdit): Promise<EventResponse> {
    return this.http
      .patch<ApiEnvelope<EventResponse>>(`/work/events/${seriesId}/occurrences`, body)
      .then(unwrap);
  }

  /** Delete one occurrence, it and every later one, or the whole series. */
  deleteOccurrence(seriesId: string, body: OccurrenceDelete): Promise<void> {
    return this.http
      .post<unknown>(`/work/events/${seriesId}/occurrences/delete`, body)
      .then(() => undefined);
  }

  /** Sprint bands with their open counts, release markers, watched tickets. */
  calendar(
    workspaceId: string,
    query: CalendarWindowQuery,
    options: RequestOptions = {}
  ): Promise<CalendarResponse> {
    return this.http
      .get<
        ApiEnvelope<CalendarResponse>
      >(`/work/workspaces/${workspaceId}/calendar?${windowQuery(query)}`, options)
      .then(unwrap);
  }
}
