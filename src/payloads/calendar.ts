/**
 * Calendar wire types (UX9): events ("Termine"), recurring series (W6-A,
 * `PATCH /work/events/:id/occurrences`, `POST …/occurrences/delete`) and
 * the sprint and release bands, `GET|POST /work/workspaces/:id/events`, `PATCH|DELETE
 * /work/events/:id`, `GET /work/workspaces/:id/calendar`.
 *
 * Generated from `rust/athenos-contracts/src/calendar.rs` by ts-rs. An
 * all-day event carries dates (`start_day`/`end_day`, both included), a
 * timed one instants (`starts_at`/`ends_at`).
 */
export type { CalendarNextSprint } from '../generated/calendar/CalendarNextSprint';
export type { CalendarRelease } from '../generated/calendar/CalendarRelease';
export type { CalendarResponse } from '../generated/calendar/CalendarResponse';
export type { CalendarSprintBand } from '../generated/calendar/CalendarSprintBand';
export type { CalendarWindowQuery } from '../generated/calendar/CalendarWindowQuery';
export type { CreateEventRequest } from '../generated/calendar/CreateEventRequest';
export type { EventEdit } from '../generated/calendar/EventEdit';
export type { EventExpected } from '../generated/calendar/EventExpected';
export type { EventResponse } from '../generated/calendar/EventResponse';
export type { EventWhen } from '../generated/calendar/EventWhen';
export type { MonthlyRule } from '../generated/calendar/MonthlyRule';
export type { OccurrenceDelete } from '../generated/calendar/OccurrenceDelete';
export type { OccurrenceEdit } from '../generated/calendar/OccurrenceEdit';
export type { RecurrenceEnd } from '../generated/calendar/RecurrenceEnd';
export type { RecurrenceFrequency } from '../generated/calendar/RecurrenceFrequency';
export type { RecurrenceRule } from '../generated/calendar/RecurrenceRule';
export type { SeriesScope } from '../generated/calendar/SeriesScope';
export type { UpdateEventRequest } from '../generated/calendar/UpdateEventRequest';
export type { Weekday } from '../generated/calendar/Weekday';
export {
  CALENDAR_MAX_WINDOW_DAYS,
  EVENT_MAX_DESCRIPTION_CHARS,
  EVENT_MAX_PARTICIPANTS,
  EVENT_MAX_SPAN_DAYS,
  EVENT_MAX_TITLE_CHARS,
  RECURRENCE_MAX_COUNT,
  RECURRENCE_MAX_INTERVAL,
  SERIES_DEFAULT_TIME_ZONE,
} from '../generated/calendar/calendar-constants';
