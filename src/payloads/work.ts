/**
 * Work wire types (`/work/*`): projects, tickets, comments, links,
 * watchers, the inbox, sprints, releases, labels, work logs and reports.
 *
 * Generated from the Rust contracts in `rust/athenos-contracts/src/work.rs`
 * by ts-rs; do not add hand-written wire shapes here. After changing a Rust
 * type, run `npm run generate` and commit `src/generated/`.
 *
 * A field that can be cleared by a PATCH is `key?: T | null`: omit the key
 * to leave the value alone, send `null` to clear it (`Patch<T>` in Rust).
 *
 * Below the re-exports: client helper types (no wire type of their own).
 */
export type { AddTicketCommentRequest } from '../generated/work/AddTicketCommentRequest';
export type { AddWorkLogRequest } from '../generated/work/AddWorkLogRequest';
export type { AssignTicketRequest } from '../generated/work/AssignTicketRequest';
export type { AssignTicketsRequest } from '../generated/work/AssignTicketsRequest';
export type { BulkSetFieldRequest } from '../generated/work/BulkSetFieldRequest';
export type { BulkUpdateConflict } from '../generated/work/BulkUpdateConflict';
export type { BulkUpdateFailure } from '../generated/work/BulkUpdateFailure';
export type { BulkUpdateTicketsRequest } from '../generated/work/BulkUpdateTicketsRequest';
export type { BulkUpdateTicketsResponse } from '../generated/work/BulkUpdateTicketsResponse';
export type { BurndownPointResponse } from '../generated/work/BurndownPointResponse';
export type { CompleteSprintRequest } from '../generated/work/CompleteSprintRequest';
export type { CreateLabelRequest } from '../generated/work/CreateLabelRequest';
export type { CreateMilestoneRequest } from '../generated/work/CreateMilestoneRequest';
export type { CreateProjectRequest } from '../generated/work/CreateProjectRequest';
export type { CreateSprintRequest } from '../generated/work/CreateSprintRequest';
export type { CreateTicketLinkRequest } from '../generated/work/CreateTicketLinkRequest';
export type { CreateTicketRequest } from '../generated/work/CreateTicketRequest';
export type { DeletedTicketResponse } from '../generated/work/DeletedTicketResponse';
export type { FlowPointResponse } from '../generated/work/FlowPointResponse';
export type { InboxEntryResponse } from '../generated/work/InboxEntryResponse';
export type { InboxQuery } from '../generated/work/InboxQuery';
export type { InboxResponse } from '../generated/work/InboxResponse';
export type { InboxReason } from '../generated/work/InboxReason';
export type { LabelAssignmentResponse } from '../generated/work/LabelAssignmentResponse';
export type { LabelResponse } from '../generated/work/LabelResponse';
export type { LinkRelation } from '../generated/work/LinkRelation';
export type { ListProjectsQuery } from '../generated/work/ListProjectsQuery';
export type { ListTicketsQuery } from '../generated/work/ListTicketsQuery';
export type { MilestoneProgress } from '../generated/work/MilestoneProgress';
export type { MilestoneResponse } from '../generated/work/MilestoneResponse';
export type { MilestoneStatus } from '../generated/work/MilestoneStatus';
export type { MoveTicketRequest } from '../generated/work/MoveTicketRequest';
export type { MoveTicketProjectRequest } from '../generated/work/MoveTicketProjectRequest';
export type { MoveTicketProjectResponse } from '../generated/work/MoveTicketProjectResponse';
export type { PerUserListQuery } from '../generated/work/PerUserListQuery';
export type { ProjectHealth } from '../generated/work/ProjectHealth';
export type { ProjectHealthMode } from '../generated/work/ProjectHealthMode';
export type { ProjectReportQuery } from '../generated/work/ProjectReportQuery';
export type { ProjectReportResponse } from '../generated/work/ProjectReportResponse';
export type { ProjectResponse } from '../generated/work/ProjectResponse';
export type { ProjectStatus } from '../generated/work/ProjectStatus';
export type { ReportSprintResponse } from '../generated/work/ReportSprintResponse';
export type { SetLabelsRequest } from '../generated/work/SetLabelsRequest';
export type { SprintBurnDownResponse } from '../generated/work/SprintBurnDownResponse';
export type { SprintCompletionStats } from '../generated/work/SprintCompletionStats';
export type { SprintResponse } from '../generated/work/SprintResponse';
export type { SprintStatus } from '../generated/work/SprintStatus';
export type { StatusShareResponse } from '../generated/work/StatusShareResponse';
export type { TicketCommentResponse } from '../generated/work/TicketCommentResponse';
export type { TicketLinkDeletedResponse } from '../generated/work/TicketLinkDeletedResponse';
export type { TicketLinkResponse } from '../generated/work/TicketLinkResponse';
export type { TicketPriority } from '../generated/work/TicketPriority';
export type { TicketResponse } from '../generated/work/TicketResponse';
export type { TicketStatus } from '../generated/work/TicketStatus';
export type { TicketWatcherListResponse } from '../generated/work/TicketWatcherListResponse';
export type { TicketWatcherResponse } from '../generated/work/TicketWatcherResponse';
export type { TriageTicketRequest } from '../generated/work/TriageTicketRequest';
export type { UpdateCustomValuesRequest } from '../generated/work/UpdateCustomValuesRequest';
export type { UpdateLabelRequest } from '../generated/work/UpdateLabelRequest';
export type { UpdateMilestoneRequest } from '../generated/work/UpdateMilestoneRequest';
export type { UpdateProjectRequest } from '../generated/work/UpdateProjectRequest';
export type { UpdateSprintRequest } from '../generated/work/UpdateSprintRequest';
export type { UpdateTicketRequest } from '../generated/work/UpdateTicketRequest';
export type { TicketEdit } from '../generated/work/TicketEdit';
export type { TicketExpected } from '../generated/work/TicketExpected';
export type { TicketProjectMove } from '../generated/work/TicketProjectMove';
export type { ProjectExpected } from '../generated/work/ProjectExpected';
export type { SprintExpected } from '../generated/work/SprintExpected';
export type { UpdateTicketStatusRequest } from '../generated/work/UpdateTicketStatusRequest';
export type { VelocityEntryResponse } from '../generated/work/VelocityEntryResponse';
export type { WatchStateResponse } from '../generated/work/WatchStateResponse';
export type { WorkActivityEntryResponse } from '../generated/work/WorkActivityEntryResponse';
export type { WorkLogListResponse } from '../generated/work/WorkLogListResponse';
export type { WorkLogResponse } from '../generated/work/WorkLogResponse';
export type { WorkScopeQuery } from '../generated/work/WorkScopeQuery';
export type { WorkStatsResponse } from '../generated/work/WorkStatsResponse';

import type { SprintResponse } from '../generated/work/SprintResponse';

// ---------- Client helpers ----------

/** The five types every workspace starts with. */
export type BuiltInTicketType = 'task' | 'bug' | 'feature' | 'support' | 'epic';
/**
 * A ticket type key. Workspaces configure their own types (see
 * `TicketTypeResponse`); the built-in keys are listed for convenience, any
 * other key of the workspace is valid too. On the wire it is a plain
 * string (`TicketResponse.ticket_type`).
 */
export type TicketType = BuiltInTicketType | (string & {});

/**
 * What `WorkApiClient.updateSprint` returns. The server answers the sprint
 * in `data` and `moved_tickets` beside it in the envelope; the client folds
 * both into this.
 */
export interface UpdateSprintResponse {
  sprint: SprintResponse;
  /** Tickets whose own due date moved with the end. */
  moved_tickets: number;
}

/** The code of the `409` a write into an archived project answers (UX-020). */
export const PROJECT_ARCHIVED_CODE = 'ATH-RES-004';
/** The i18n key a client renders for {@link PROJECT_ARCHIVED_CODE}. */
export const PROJECT_ARCHIVED_MESSAGE_KEY = 'errors.project_archived';
