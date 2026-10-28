/**
 * The work timer (W6-B): one running or paused timer per person, kept by
 * the server. `GET /work/timer`, `POST /work/timer/{start,pause,resume,
 * switch,finish,discard}`.
 *
 * Generated from `rust/athenos-contracts/src/timer.rs` by ts-rs. Time comes
 * from the server's clock: `elapsed_seconds` is the time counted at the
 * answer's `server_now`; a client ticks from `started_at` and corrects for
 * its own clock with `server_now`.
 */
export type { FinishTimerRequest } from '../generated/timer/FinishTimerRequest';
export type { StartTimerRequest } from '../generated/timer/StartTimerRequest';
export type { SwitchTimerRequest } from '../generated/timer/SwitchTimerRequest';
export type { TimerAnswer } from '../generated/timer/TimerAnswer';
export type { TimerCarry } from '../generated/timer/TimerCarry';
export type { TimerDone } from '../generated/timer/TimerDone';
export type { TimerExpected } from '../generated/timer/TimerExpected';
export type { TimerHasTimeDetails } from '../generated/timer/TimerHasTimeDetails';
export type { TimerPrevious } from '../generated/timer/TimerPrevious';
export type { TimerResponse } from '../generated/timer/TimerResponse';
export type { TimerStateResponse } from '../generated/timer/TimerStateResponse';
export type { TimerStatusOutcome } from '../generated/timer/TimerStatusOutcome';
export type { TimerStatusRefusal } from '../generated/timer/TimerStatusRefusal';
export type { TimerWriteRequest } from '../generated/timer/TimerWriteRequest';
export {
  TIME_TRACKING_FLAG,
  TIMER_HAS_TIME_REASON,
  TIMER_MAX_SECONDS,
} from '../generated/timer/timer-constants';
