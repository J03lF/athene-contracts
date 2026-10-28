/**
 * A project at a glance (B2, W7-P): `GET /work/projects/stats` with
 * `workspace_id` or `project_id`. Tickets per status category, points,
 * the people with open work, overdue and due-soon counts, the last change
 * — counted by the server, so an overview never reads the tickets.
 * Generated from `rust/athene-contracts/src/project_stats.rs`.
 */
export type { CategoryCounts } from '../generated/work/CategoryCounts';
export type { DueCounts } from '../generated/work/DueCounts';
export type { PointCounts } from '../generated/work/PointCounts';
export type { ProjectPerson } from '../generated/work/ProjectPerson';
export type { ProjectStats } from '../generated/work/ProjectStats';
export type { ProjectStatsQuery } from '../generated/work/ProjectStatsQuery';
export type { ProjectStatsResponse } from '../generated/work/ProjectStatsResponse';

/** The most people one project's stats name. */
export const PROJECT_STATS_MAX_PEOPLE = 8;
/** Days ahead that count as "due soon". */
export const PROJECT_STATS_DUE_SOON_DAYS = 7;
/** Days back that count as "recent". */
export const PROJECT_STATS_RECENT_DAYS = 7;
