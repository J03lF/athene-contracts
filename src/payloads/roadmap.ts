/**
 * The roadmap (W7-R): the timeline of a workspace or one project.
 * `GET /work/workspaces/{id}/roadmap?project_id=&parent_id=`.
 *
 * Generated from `rust/athene-contracts/src/roadmap.rs` by ts-rs. A bar is
 * moved with the ordinary ticket edit (`start_date`, `due_date` and their
 * base in `expected`).
 */
export type { RoadmapChildCounts } from '../generated/roadmap/RoadmapChildCounts';
export type { RoadmapItem } from '../generated/roadmap/RoadmapItem';
export type { RoadmapLink } from '../generated/roadmap/RoadmapLink';
export type { RoadmapQuery } from '../generated/roadmap/RoadmapQuery';
export type { RoadmapRelease } from '../generated/roadmap/RoadmapRelease';
export type { RoadmapResponse } from '../generated/roadmap/RoadmapResponse';
export type { RoadmapSprint } from '../generated/roadmap/RoadmapSprint';
export { ROADMAP_MAX_ITEMS } from '../generated/roadmap/roadmap-constants';
