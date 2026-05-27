// Activity feed payloads (synced with rust crate `activity.rs`).

export interface ActivityEventResponse {
  id: string;
  workspace_id: string;
  actor_id: string | null;
  /** Verb (e.g., `ticket_created`, `ticket_status_changed`, `sprint_started`). */
  action: string;
  /** `ticket`, `project`, `sprint`, `milestone`, ... */
  target_type: string;
  target_id: string;
  detail: string | null;
  metadata: Record<string, unknown> | null;
  /** ISO-8601 (RFC 3339). */
  created_at: string;
}
