import type {
  FinishTimerRequest,
  StartTimerRequest,
  SwitchTimerRequest,
  TimerAnswer,
  TimerStateResponse,
  TimerWriteRequest,
} from '../payloads/timer';
import type { AtheneHttpClient, RequestOptions } from '../http';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

/**
 * The caller's work timer (W6-B), kept by the server.
 *
 * Every write sends `expected` (the version seen, `null` for "no timer");
 * a stale one is `409 ATH-RES-003` with the timer as it is now in
 * `details.current`. A start on another ticket, or a switch without
 * `previous`, while the timer holds time is `409` with
 * `details.reason === TIMER_HAS_TIME_REASON`. While the platform switches
 * time tracking off every route is `403 ATH-FEAT-001`.
 */
export class TimerApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  /** The caller's timer (or `null`) and the server's clock. */
  get(options: RequestOptions = {}): Promise<TimerStateResponse> {
    return this.http.get<ApiEnvelope<TimerStateResponse>>('/work/timer', options).then(unwrap);
  }

  start(body: StartTimerRequest): Promise<TimerAnswer> {
    return this.write('start', body);
  }

  pause(body: TimerWriteRequest): Promise<TimerAnswer> {
    return this.write('pause', body);
  }

  resume(body: TimerWriteRequest): Promise<TimerAnswer> {
    return this.write('resume', body);
  }

  /** Move to another ticket; `previous` books or discards the time held. */
  switch(body: SwitchTimerRequest): Promise<TimerAnswer> {
    return this.write('switch', body);
  }

  /** Book the time as a work log and clear the timer; optionally "done". */
  finish(body: FinishTimerRequest): Promise<TimerAnswer> {
    return this.write('finish', body);
  }

  discard(body: TimerWriteRequest): Promise<TimerAnswer> {
    return this.write('discard', body);
  }

  private write(action: string, body: unknown): Promise<TimerAnswer> {
    return this.http.post<ApiEnvelope<TimerAnswer>>(`/work/timer/${action}`, body).then(unwrap);
  }
}
