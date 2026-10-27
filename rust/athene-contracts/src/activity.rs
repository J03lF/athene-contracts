//! Activity feed contracts.
//!
//! The activity feed is UI-facing (separate from the security audit log).
//! See `ActivityRepository` in the athene crate for the persistence layer.
//! The TypeScript in `src/generated/activity/` is generated from these (see
//! `tests/ts_bindings.rs`).

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use ts_rs::TS;
use utoipa::ToSchema;

/// One feed entry: `GET /work/workspaces/:id/activity`,
/// `GET /work/tickets/:id/activity` and the home page's recent activity.
///
/// `detail` carries only user-authored text (a ticket title). Anything
/// machine-derived travels in `metadata`, so the client renders it in the
/// reader's language instead of printing a server-composed sentence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema, TS)]
#[ts(export_to = "activity/")]
pub struct ActivityEventResponse {
    pub id: String,
    pub workspace_id: String,
    /// `null` for system or automation events with no human actor.
    pub actor_id: Option<String>,
    /// Verb (e.g., `ticket_created`, `ticket_status_changed`,
    /// `sprint_started`): an [`ActivityAction`] for everything this server
    /// writes. A string on the wire, because the feed holds history: render
    /// a verb the reader's build does not know as a generic entry.
    pub action: String,
    /// `ticket`, `project`, `sprint`, `milestone`, ...
    pub target_type: String,
    pub target_id: String,
    /// Short user-authored text (e.g. a ticket title).
    pub detail: Option<String>,
    /// Structured payload for the verb. `ticket_status_changed` carries
    /// `{ from, to }` in the ticket-status vocabulary; `ticket_assigned`
    /// carries `{ assignee_id }`; `ticket_updated` carries
    /// `{ fields: string[] }`; `ticket_linked` carries
    /// `{ relation, to_ticket_id }`.
    #[ts(type = "Record<string, unknown> | null")]
    pub metadata: Option<JsonValue>,
    /// ISO-8601 (RFC 3339).
    pub created_at: String,
}

pub type ActivityListResponse = crate::ListResponse<ActivityEventResponse>;

/// What happened, as an activity entry names it (BE-17, Q-058, step 3.10).
///
/// A closed set: the core records only these (`ActivityService` and the
/// write log take this type, so a new verb is a new variant here), the
/// TypeScript union `ActivityAction` is generated from it, and the web
/// narrates it in an exhaustive `switch`. On the wire
/// (`ActivityEventResponse.action`) the verb stays a string, because the
/// feed is history: entries written by an older or newer server than the
/// reader's must still render, as a generic line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "activity/")]
pub enum ActivityAction {
    TicketCreated,
    TicketDeleted,
    TicketStatusChanged,
    TicketAssigned,
    TicketTriaged,
    TicketCommented,
    TicketCommentEdited,
    TicketMentioned,
    TicketUpdated,
    TicketCustomValuesUpdated,
    TicketLinked,
    TicketLabeled,
    TicketWorkLogged,
    TicketMovedProject,
    TicketSprintAssigned,
    TicketSprintUnassigned,
    TicketDueFollowedSprint,
    /// A file put onto the ticket or one of its comments (W4-B). `detail`:
    /// the file name; metadata `{ attachment_id, comment_id, content_type,
    /// size_bytes }`.
    TicketAttachmentAdded,
    /// A file moved to the ticket's trash.
    TicketAttachmentRemoved,
    /// A file back from the trash.
    TicketAttachmentRestored,
    /// A file removed from the trash for good.
    TicketAttachmentPurged,
    /// A file renamed; metadata also `{ from }`.
    TicketAttachmentRenamed,
    /// The ticket's "Release-Notiz" set, changed or removed (W4-H).
    /// metadata `{ set: bool }`; the text itself is not copied.
    TicketReleaseNoteUpdated,
    /// The ticket's due date (its own, else its sprint's end) is within
    /// the workspace's "due soon" window (W4-H, written by the
    /// notification job, no actor). metadata `{ due_at, source: own |
    /// sprint, hours }`; once per ticket and due date.
    TicketDueSoon,
    ProjectUpdated,
    ProjectCustomValuesUpdated,
    SprintCreated,
    SprintStarted,
    SprintUpdated,
    SprintRescheduled,
    SprintCompleted,
    SprintDeleted,
    SprintAssignedTickets,
    SprintUnassignedTickets,
    MilestoneCreated,
    MilestoneReleased,
    MilestoneReopened,
    MilestoneDeleted,
    /// A release's notes were generated into its drafts (W4-H). Target the
    /// release; metadata `{ variants: [...] }`.
    ReleaseNotesGenerated,
    /// A release's notes settings or drafts changed (W4-H). metadata
    /// `{ fields: [...] }`.
    ReleaseNotesUpdated,
    /// A workspace's release-note template saved or reset (W4-H). Target
    /// the workspace; metadata `{ reset: bool }`.
    ReleaseNoteTemplateUpdated,
    /// A workspace's notification defaults changed (W4-H). Target the
    /// workspace; metadata `{ fields: [...] }`.
    NotificationDefaultsUpdated,
    DocCreated,
    DocUpdated,
    DocDeleted,
    FieldDefinitionCreated,
    FieldDefinitionUpdated,
    FieldDefinitionDeleted,
    FieldDefinitionsReordered,
    FieldDefinitionArchived,
    FieldDefinitionRestored,
    FieldAssignmentCreated,
    FieldAssignmentUpdated,
    FieldAssignmentDeleted,
    FieldAttached,
    FieldDetached,
    FieldSuggested,
    FieldSuggestionDecided,
    TicketTypeCreated,
    TicketTypeUpdated,
    TicketTypeDeleted,
    LabelCreated,
    LabelUpdated,
    LabelDeleted,
    TeamCreated,
    TeamUpdated,
    TeamDeleted,
    TeamMembersAdded,
    TeamMemberRoleChanged,
    TeamMemberRemoved,
    TeamMemberLeft,
    MemberRoleChanged,
    MemberRemoved,
    MemberLeft,
    InviteRevoked,
    RoleCreated,
    RoleUpdated,
    RoleDeleted,
    OverrideSet,
    OverrideCleared,
    /// A status the workspace added (W4-F). Target `workflow_status`,
    /// `detail` its name; metadata `{ key, category }`.
    WorkflowStatusCreated,
    /// Renamed, recoloured, recategorised, restored from the archive or
    /// hidden/shown in a project; metadata also `{ fields }` (and
    /// `{ project_id, hidden }` for a project's hidden statuses).
    WorkflowStatusUpdated,
    /// Archived: no new ticket enters it.
    WorkflowStatusArchived,
    /// Deleted; metadata also `{ moved_tickets, move_to, removed_queues }`.
    WorkflowStatusDeleted,
}

impl ActivityAction {
    /// Every verb, in declaration order.
    pub const ALL: &'static [Self] = &[
        Self::TicketCreated,
        Self::TicketDeleted,
        Self::TicketStatusChanged,
        Self::TicketAssigned,
        Self::TicketTriaged,
        Self::TicketCommented,
        Self::TicketCommentEdited,
        Self::TicketMentioned,
        Self::TicketUpdated,
        Self::TicketCustomValuesUpdated,
        Self::TicketLinked,
        Self::TicketLabeled,
        Self::TicketWorkLogged,
        Self::TicketMovedProject,
        Self::TicketSprintAssigned,
        Self::TicketSprintUnassigned,
        Self::TicketDueFollowedSprint,
        Self::TicketAttachmentAdded,
        Self::TicketAttachmentRemoved,
        Self::TicketAttachmentRestored,
        Self::TicketAttachmentPurged,
        Self::TicketAttachmentRenamed,
        Self::TicketReleaseNoteUpdated,
        Self::TicketDueSoon,
        Self::ProjectUpdated,
        Self::ProjectCustomValuesUpdated,
        Self::SprintCreated,
        Self::SprintStarted,
        Self::SprintUpdated,
        Self::SprintRescheduled,
        Self::SprintCompleted,
        Self::SprintDeleted,
        Self::SprintAssignedTickets,
        Self::SprintUnassignedTickets,
        Self::MilestoneCreated,
        Self::MilestoneReleased,
        Self::MilestoneReopened,
        Self::MilestoneDeleted,
        Self::ReleaseNotesGenerated,
        Self::ReleaseNotesUpdated,
        Self::ReleaseNoteTemplateUpdated,
        Self::NotificationDefaultsUpdated,
        Self::DocCreated,
        Self::DocUpdated,
        Self::DocDeleted,
        Self::FieldDefinitionCreated,
        Self::FieldDefinitionUpdated,
        Self::FieldDefinitionDeleted,
        Self::FieldDefinitionsReordered,
        Self::FieldDefinitionArchived,
        Self::FieldDefinitionRestored,
        Self::FieldAssignmentCreated,
        Self::FieldAssignmentUpdated,
        Self::FieldAssignmentDeleted,
        Self::FieldAttached,
        Self::FieldDetached,
        Self::FieldSuggested,
        Self::FieldSuggestionDecided,
        Self::TicketTypeCreated,
        Self::TicketTypeUpdated,
        Self::TicketTypeDeleted,
        Self::LabelCreated,
        Self::LabelUpdated,
        Self::LabelDeleted,
        Self::TeamCreated,
        Self::TeamUpdated,
        Self::TeamDeleted,
        Self::TeamMembersAdded,
        Self::TeamMemberRoleChanged,
        Self::TeamMemberRemoved,
        Self::TeamMemberLeft,
        Self::MemberRoleChanged,
        Self::MemberRemoved,
        Self::MemberLeft,
        Self::InviteRevoked,
        Self::RoleCreated,
        Self::RoleUpdated,
        Self::RoleDeleted,
        Self::OverrideSet,
        Self::OverrideCleared,
        Self::WorkflowStatusCreated,
        Self::WorkflowStatusUpdated,
        Self::WorkflowStatusArchived,
        Self::WorkflowStatusDeleted,
    ];

    /// The verb on the wire and in the database.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TicketCreated => "ticket_created",
            Self::TicketDeleted => "ticket_deleted",
            Self::TicketStatusChanged => "ticket_status_changed",
            Self::TicketAssigned => "ticket_assigned",
            Self::TicketTriaged => "ticket_triaged",
            Self::TicketCommented => "ticket_commented",
            Self::TicketCommentEdited => "ticket_comment_edited",
            Self::TicketMentioned => "ticket_mentioned",
            Self::TicketUpdated => "ticket_updated",
            Self::TicketCustomValuesUpdated => "ticket_custom_values_updated",
            Self::TicketLinked => "ticket_linked",
            Self::TicketLabeled => "ticket_labeled",
            Self::TicketWorkLogged => "ticket_work_logged",
            Self::TicketMovedProject => "ticket_moved_project",
            Self::TicketSprintAssigned => "ticket_sprint_assigned",
            Self::TicketSprintUnassigned => "ticket_sprint_unassigned",
            Self::TicketDueFollowedSprint => "ticket_due_followed_sprint",
            Self::TicketAttachmentAdded => "ticket_attachment_added",
            Self::TicketAttachmentRemoved => "ticket_attachment_removed",
            Self::TicketAttachmentRestored => "ticket_attachment_restored",
            Self::TicketAttachmentPurged => "ticket_attachment_purged",
            Self::TicketAttachmentRenamed => "ticket_attachment_renamed",
            Self::TicketReleaseNoteUpdated => "ticket_release_note_updated",
            Self::TicketDueSoon => "ticket_due_soon",
            Self::ProjectUpdated => "project_updated",
            Self::ProjectCustomValuesUpdated => "project_custom_values_updated",
            Self::SprintCreated => "sprint_created",
            Self::SprintStarted => "sprint_started",
            Self::SprintUpdated => "sprint_updated",
            Self::SprintRescheduled => "sprint_rescheduled",
            Self::SprintCompleted => "sprint_completed",
            Self::SprintDeleted => "sprint_deleted",
            Self::SprintAssignedTickets => "sprint_assigned_tickets",
            Self::SprintUnassignedTickets => "sprint_unassigned_tickets",
            Self::MilestoneCreated => "milestone_created",
            Self::MilestoneReleased => "milestone_released",
            Self::MilestoneReopened => "milestone_reopened",
            Self::MilestoneDeleted => "milestone_deleted",
            Self::ReleaseNotesGenerated => "release_notes_generated",
            Self::ReleaseNotesUpdated => "release_notes_updated",
            Self::ReleaseNoteTemplateUpdated => "release_note_template_updated",
            Self::NotificationDefaultsUpdated => "notification_defaults_updated",
            Self::DocCreated => "doc_created",
            Self::DocUpdated => "doc_updated",
            Self::DocDeleted => "doc_deleted",
            Self::FieldDefinitionCreated => "field_definition_created",
            Self::FieldDefinitionUpdated => "field_definition_updated",
            Self::FieldDefinitionDeleted => "field_definition_deleted",
            Self::FieldDefinitionsReordered => "field_definitions_reordered",
            Self::FieldDefinitionArchived => "field_definition_archived",
            Self::FieldDefinitionRestored => "field_definition_restored",
            Self::FieldAssignmentCreated => "field_assignment_created",
            Self::FieldAssignmentUpdated => "field_assignment_updated",
            Self::FieldAssignmentDeleted => "field_assignment_deleted",
            Self::FieldAttached => "field_attached",
            Self::FieldDetached => "field_detached",
            Self::FieldSuggested => "field_suggested",
            Self::FieldSuggestionDecided => "field_suggestion_decided",
            Self::TicketTypeCreated => "ticket_type_created",
            Self::TicketTypeUpdated => "ticket_type_updated",
            Self::TicketTypeDeleted => "ticket_type_deleted",
            Self::LabelCreated => "label_created",
            Self::LabelUpdated => "label_updated",
            Self::LabelDeleted => "label_deleted",
            Self::TeamCreated => "team_created",
            Self::TeamUpdated => "team_updated",
            Self::TeamDeleted => "team_deleted",
            Self::TeamMembersAdded => "team_members_added",
            Self::TeamMemberRoleChanged => "team_member_role_changed",
            Self::TeamMemberRemoved => "team_member_removed",
            Self::TeamMemberLeft => "team_member_left",
            Self::MemberRoleChanged => "member_role_changed",
            Self::MemberRemoved => "member_removed",
            Self::MemberLeft => "member_left",
            Self::InviteRevoked => "invite_revoked",
            Self::RoleCreated => "role_created",
            Self::RoleUpdated => "role_updated",
            Self::RoleDeleted => "role_deleted",
            Self::OverrideSet => "override_set",
            Self::OverrideCleared => "override_cleared",
            Self::WorkflowStatusCreated => "workflow_status_created",
            Self::WorkflowStatusUpdated => "workflow_status_updated",
            Self::WorkflowStatusArchived => "workflow_status_archived",
            Self::WorkflowStatusDeleted => "workflow_status_deleted",
        }
    }

    /// The verb `value` names, if this build knows it.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|action| action.as_str() == value)
    }
}

impl std::fmt::Display for ActivityAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::ActivityAction;

    #[test]
    fn every_verb_is_spelled_once_and_as_serde_spells_it() {
        let mut seen = std::collections::HashSet::new();
        for action in ActivityAction::ALL {
            assert!(seen.insert(action.as_str()), "{action} twice");
            let wire = serde_json::to_value(action).unwrap();
            assert_eq!(wire, serde_json::json!(action.as_str()));
            assert_eq!(ActivityAction::parse(action.as_str()), Some(*action));
        }
        assert_eq!(ActivityAction::parse("status_changed"), None);
    }
}
