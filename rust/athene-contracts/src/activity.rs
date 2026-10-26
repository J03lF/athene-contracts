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
    DocCreated,
    DocUpdated,
    DocDeleted,
    FieldDefinitionCreated,
    FieldDefinitionUpdated,
    FieldDefinitionDeleted,
    FieldDefinitionsReordered,
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
        Self::DocCreated,
        Self::DocUpdated,
        Self::DocDeleted,
        Self::FieldDefinitionCreated,
        Self::FieldDefinitionUpdated,
        Self::FieldDefinitionDeleted,
        Self::FieldDefinitionsReordered,
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
            Self::DocCreated => "doc_created",
            Self::DocUpdated => "doc_updated",
            Self::DocDeleted => "doc_deleted",
            Self::FieldDefinitionCreated => "field_definition_created",
            Self::FieldDefinitionUpdated => "field_definition_updated",
            Self::FieldDefinitionDeleted => "field_definition_deleted",
            Self::FieldDefinitionsReordered => "field_definitions_reordered",
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
