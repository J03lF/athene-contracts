//! Watchers and notifications (W4-H): what reaches a person, through which
//! channel and how often, who starts watching a ticket on their own, and
//! the digests.
//!
//! **Events.** Every ticket activity verb ([`crate::activity::ActivityAction`])
//! belongs to one [`NotificationEvent`] ([`NotificationEvent::of_action`],
//! exhaustive) — comment, mention, status change, assignment, queue arrival
//! (a status change into a queue of the reader's role), due soon,
//! attachment, field change. A verb without one (a sprint, a doc) never
//! reaches the inbox through this layer.
//!
//! **Delivery.** Per person, per event and per channel ([`NotificationChannel`]:
//! the inbox, always there, and email, when the core can send mail):
//! [`Delivery`] `immediate`, `daily`, `weekly` or `off`. A person's own choice
//! wins, else their workspace's default (owners and admins set it), else
//! the product's ([`default_delivery`]). `daily` and `weekly` collect the
//! events into one digest per person: one inbox entry and/or one mail.
//!
//! **Auto-watch.** [`AutoWatchRule`]: on create (the reporter), on assign
//! (the assignee), on comment (the author), on mention (the mentioned), on
//! take (from a queue). Each on or off per person, with the workspace's
//! default and the product's ([`default_auto_watch`]: all on) behind it.
//!
//! Routes (every response is `{ "data": … }`, bodies refuse unknown fields):
//! - `GET /user/notifications[?workspace_id=]` — the caller's settings, the
//!   effective values against the named workspace's defaults;
//!   `PUT /user/notifications` ([`UpdateNotificationSettingsRequest`], key by
//!   key, `null` = back to the default).
//! - `GET /user/notifications/digests[?limit=]` — the caller's digests
//!   ([`DigestListResponse`]), filtered by what they may read now.
//! - `GET /work/workspaces/:workspace_id/notification-defaults` (every
//!   member), `PUT` (`workspace.settings.edit`).
//! - `GET /work/tickets/:id/watchers` carries `count`; `PUT`/`DELETE
//!   /work/tickets/:id/watchers/me` as before.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::activity::ActivityAction;
use crate::Patch;

/// What a person can be told about.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "notifications/")]
pub enum NotificationEvent {
    Comment,
    Mention,
    StatusChange,
    Assignment,
    /// The ticket entered a queue of the reader's role (custom roles C1).
    QueueArrival,
    /// The ticket's due date (its own, else its sprint's end) is near.
    DueSoon,
    Attachment,
    FieldChange,
}

impl NotificationEvent {
    /// Every event, in the order the settings show them.
    pub const ALL: [Self; 8] = [
        Self::Comment,
        Self::Mention,
        Self::StatusChange,
        Self::Assignment,
        Self::QueueArrival,
        Self::DueSoon,
        Self::Attachment,
        Self::FieldChange,
    ];

    /// The key on the wire and in the stored settings.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Comment => "comment",
            Self::Mention => "mention",
            Self::StatusChange => "status_change",
            Self::Assignment => "assignment",
            Self::QueueArrival => "queue_arrival",
            Self::DueSoon => "due_soon",
            Self::Attachment => "attachment",
            Self::FieldChange => "field_change",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.as_str() == value)
    }

    /// The event an activity verb is, for a reader who is *not* reached
    /// through a queue (a status change into the reader's queue is
    /// [`Self::QueueArrival`]; the reader's involvement decides that).
    /// Exhaustive: a new verb has to say what it is.
    pub const fn of_action(action: ActivityAction) -> Option<Self> {
        use ActivityAction as A;
        match action {
            A::TicketCommented | A::TicketCommentEdited => Some(Self::Comment),
            A::TicketMentioned => Some(Self::Mention),
            A::TicketStatusChanged => Some(Self::StatusChange),
            // A new ticket reaches someone other than its author only as
            // its assignee.
            A::TicketCreated | A::TicketAssigned => Some(Self::Assignment),
            A::TicketDueSoon => Some(Self::DueSoon),
            A::TicketAttachmentAdded
            | A::TicketAttachmentRemoved
            | A::TicketAttachmentRestored
            | A::TicketAttachmentPurged
            | A::TicketAttachmentRenamed => Some(Self::Attachment),
            A::TicketTriaged
            | A::TicketUpdated
            | A::TicketCustomValuesUpdated
            | A::TicketLinked
            | A::TicketLabeled
            | A::TicketWorkLogged
            | A::TicketMovedProject
            | A::TicketSprintAssigned
            | A::TicketSprintUnassigned
            | A::TicketDueFollowedSprint
            | A::TicketReleaseNoteUpdated => Some(Self::FieldChange),
            // The ticket is gone: nobody can open it any more.
            A::TicketDeleted
            | A::ProjectUpdated
            | A::ProjectCustomValuesUpdated
            | A::SprintCreated
            | A::SprintStarted
            | A::SprintUpdated
            | A::SprintRescheduled
            | A::SprintCompleted
            | A::SprintDeleted
            | A::SprintAssignedTickets
            | A::SprintUnassignedTickets
            | A::MilestoneCreated
            | A::MilestoneReleased
            | A::MilestoneReopened
            | A::MilestoneDeleted
            | A::ReleaseNotesGenerated
            | A::ReleaseNotesUpdated
            | A::ReleaseNoteTemplateUpdated
            | A::NotificationDefaultsUpdated
            | A::DocCreated
            | A::DocUpdated
            | A::DocDeleted
            | A::FieldDefinitionCreated
            | A::FieldDefinitionUpdated
            | A::FieldDefinitionDeleted
            | A::FieldDefinitionsReordered
            | A::FieldDefinitionArchived
            | A::FieldDefinitionRestored
            | A::FieldAssignmentCreated
            | A::FieldAssignmentUpdated
            | A::FieldAssignmentDeleted
            | A::FieldAttached
            | A::FieldDetached
            | A::FieldSuggested
            | A::FieldSuggestionDecided
            | A::TicketTypeCreated
            | A::TicketTypeUpdated
            | A::TicketTypeDeleted
            | A::LabelCreated
            | A::LabelUpdated
            | A::LabelDeleted
            | A::TeamCreated
            | A::TeamUpdated
            | A::TeamDeleted
            | A::TeamMembersAdded
            | A::TeamMemberRoleChanged
            | A::TeamMemberRemoved
            | A::TeamMemberLeft
            | A::MemberRoleChanged
            | A::MemberRolesChanged
            | A::MemberRemoved
            | A::MemberLeft
            | A::InviteRevoked
            | A::RoleCreated
            | A::RoleUpdated
            | A::RoleDeleted
            | A::OverrideSet
            | A::OverrideCleared
            // A workspace's statuses (W4-F) are its settings, no ticket's news.
            | A::WorkflowStatusCreated
            | A::WorkflowStatusUpdated
            | A::WorkflowStatusArchived
            | A::WorkflowStatusDeleted => None,
        }
    }
}

/// How often something reaches a channel.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "notifications/")]
pub enum Delivery {
    /// As it happens (the inbox at once; a mail within a minute or two).
    Immediate,
    /// In the daily digest.
    Daily,
    /// In the weekly digest.
    Weekly,
    Off,
}

impl Delivery {
    pub const ALL: [Self; 4] = [Self::Immediate, Self::Daily, Self::Weekly, Self::Off];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Immediate => "immediate",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Off => "off",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.as_str() == value)
    }
}

/// Where something reaches a person.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "notifications/")]
pub enum NotificationChannel {
    /// Always available.
    Inbox,
    /// Only while the core can send mail (`email_available`).
    Email,
}

impl NotificationChannel {
    pub const ALL: [Self; 2] = [Self::Inbox, Self::Email];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inbox => "inbox",
            Self::Email => "email",
        }
    }
}

/// The product's delivery when neither the person nor their workspace chose:
/// everything into the inbox at once; by mail what is addressed to the
/// person at once, the conversation once a day, the noise not at all.
pub const fn default_delivery(event: NotificationEvent, channel: NotificationChannel) -> Delivery {
    use NotificationEvent as E;
    match channel {
        NotificationChannel::Inbox => Delivery::Immediate,
        NotificationChannel::Email => match event {
            E::Mention | E::Assignment | E::QueueArrival | E::DueSoon => Delivery::Immediate,
            E::Comment | E::StatusChange => Delivery::Daily,
            E::Attachment | E::FieldChange => Delivery::Off,
        },
    }
}

/// When someone starts watching a ticket without pressing "watch".
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "notifications/")]
pub enum AutoWatchRule {
    /// The reporter, creating the ticket.
    Create,
    /// The assignee, on create or on assignment.
    Assign,
    /// The author of a comment.
    Comment,
    /// Who a comment mentions.
    Mention,
    /// Who takes the ticket from a queue.
    Take,
}

impl AutoWatchRule {
    pub const ALL: [Self; 5] = [
        Self::Create,
        Self::Assign,
        Self::Comment,
        Self::Mention,
        Self::Take,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Assign => "assign",
            Self::Comment => "comment",
            Self::Mention => "mention",
            Self::Take => "take",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.as_str() == value)
    }

    /// The key of `user_settings.preferences` that switched it off before
    /// W4-H (`false` only), read after the person's own choice so an older
    /// web keeps working. `take` followed the assignment switch.
    pub const fn legacy_preference_key(self) -> Option<&'static str> {
        match self {
            Self::Create => Some("auto_watch_created"),
            Self::Assign | Self::Take => Some("auto_watch_assigned"),
            Self::Comment | Self::Mention => None,
        }
    }
}

/// The product's auto-watch: every rule on (what the people involved most
/// need is to hear what happens next).
pub const fn default_auto_watch(_rule: AutoWatchRule) -> bool {
    true
}

/// Where an effective value comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "notifications/")]
pub enum SettingSource {
    /// The person chose it.
    Own,
    /// The person's choice from before W4-H (`preferences.auto_watch_*`).
    Legacy,
    /// The workspace's default.
    Workspace,
    /// The product's default.
    Product,
}

/// A digest's rhythm.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "notifications/")]
pub enum DigestCadence {
    Daily,
    Weekly,
}

impl DigestCadence {
    pub const ALL: [Self; 2] = [Self::Daily, Self::Weekly];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
        }
    }

    /// The delivery that puts an event into this digest.
    pub const fn delivery(self) -> Delivery {
        match self {
            Self::Daily => Delivery::Daily,
            Self::Weekly => Delivery::Weekly,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == value)
    }
}

/// The time of day a digest goes out when none is set (`HH:MM`, the
/// person's zone).
pub const DIGEST_DEFAULT_TIME: &str = "08:00";
/// The weekly digest's day when none is set (ISO: 1 = Monday).
pub const DIGEST_DEFAULT_WEEKDAY: u8 = 1;
/// The window "due soon" looks ahead when the workspace set none, in hours.
pub const DUE_SOON_DEFAULT_HOURS: u32 = 24;
/// The smallest and largest "due soon" window, in hours (two weeks).
pub const DUE_SOON_MIN_HOURS: u32 = 1;
pub const DUE_SOON_MAX_HOURS: u32 = 336;
/// The most digests one read returns, and its default.
pub const DIGEST_LIST_MAX: usize = 50;
pub const DIGEST_LIST_DEFAULT: usize = 20;
/// The most events one digest holds; past it the digest says how many
/// more there were (`omitted`).
pub const DIGEST_MAX_EVENTS: usize = 200;

// ---------------------------------------------------------------------------
// Responses
// ---------------------------------------------------------------------------

/// One channel's delivery of one event, as it applies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "notifications/")]
pub struct EffectiveDelivery {
    pub value: Delivery,
    pub source: SettingSource,
    /// What the person set themselves; `null` = they follow the default.
    pub own: Option<Delivery>,
    /// What applies without their choice (workspace, else product).
    pub default: Delivery,
}

/// One event's row of the settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "notifications/")]
pub struct EventSettingResponse {
    pub event: NotificationEvent,
    pub inbox: EffectiveDelivery,
    pub email: EffectiveDelivery,
}

/// One auto-watch rule, as it applies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "notifications/")]
pub struct AutoWatchSettingResponse {
    pub rule: AutoWatchRule,
    pub enabled: bool,
    pub source: SettingSource,
    /// The person's own choice; `null` = the default.
    pub own: Option<bool>,
    pub default: bool,
}

/// When the digests go out, in the person's zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "notifications/")]
pub struct DigestScheduleResponse {
    /// `HH:MM`.
    pub daily_time: String,
    /// ISO weekday, 1 (Monday) … 7.
    pub weekly_day: u8,
    /// `HH:MM`.
    pub weekly_time: String,
    /// The IANA zone the times are read in: the person's setting
    /// (`/user/settings` `timezone`), else the server's default.
    pub timezone: String,
    /// The person has no zone of their own; `timezone` is the server's.
    pub timezone_is_default: bool,
}

/// `GET /user/notifications`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "notifications/")]
pub struct NotificationSettingsResponse {
    /// Every [`NotificationEvent`], in [`NotificationEvent::ALL`] order.
    pub events: Vec<EventSettingResponse>,
    /// Every [`AutoWatchRule`], in [`AutoWatchRule::ALL`] order.
    pub auto_watch: Vec<AutoWatchSettingResponse>,
    pub digest: DigestScheduleResponse,
    /// The core can send mail at all; without it the email column is
    /// shown disabled and nothing is mailed.
    pub email_available: bool,
    /// The person's master switches (`/user/settings`): mail only while
    /// both `notifications_enabled` and `email_notifications` hold.
    pub email_enabled: bool,
    /// The workspace whose defaults the effective values are read against
    /// (`?workspace_id=`), if one was named.
    pub workspace_id: Option<String>,
}

/// One rule of the workspace defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "notifications/")]
pub struct DefaultDelivery {
    pub value: Delivery,
    /// The workspace set it; otherwise `value` is the product's.
    pub set: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "notifications/")]
pub struct EventDefaultResponse {
    pub event: NotificationEvent,
    pub inbox: DefaultDelivery,
    pub email: DefaultDelivery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "notifications/")]
pub struct AutoWatchDefaultResponse {
    pub rule: AutoWatchRule,
    pub enabled: bool,
    pub set: bool,
}

/// `GET /work/workspaces/:workspace_id/notification-defaults`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "notifications/")]
pub struct WorkspaceNotificationDefaultsResponse {
    pub workspace_id: String,
    pub events: Vec<EventDefaultResponse>,
    pub auto_watch: Vec<AutoWatchDefaultResponse>,
    /// The "due soon" window, in hours.
    pub due_soon_hours: u32,
    pub due_soon_set: bool,
    /// The caller may change them (`workspace.settings.edit`).
    pub editable: bool,
}

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

/// A change to one event's channels: absent = unchanged, `null` = back to
/// the default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "notifications/")]
pub struct EventDeliveryPatch {
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<Delivery>)]
    pub inbox: Patch<Delivery>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<Delivery>)]
    pub email: Patch<Delivery>,
}

/// Changes to the events, one optional entry each.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "notifications/", optional_fields)]
pub struct NotificationEventsPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<EventDeliveryPatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mention: Option<EventDeliveryPatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_change: Option<EventDeliveryPatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignment: Option<EventDeliveryPatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_arrival: Option<EventDeliveryPatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_soon: Option<EventDeliveryPatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment: Option<EventDeliveryPatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_change: Option<EventDeliveryPatch>,
}

impl NotificationEventsPatch {
    /// The entries present, with their event.
    pub fn entries(&self) -> Vec<(NotificationEvent, &EventDeliveryPatch)> {
        use NotificationEvent as E;
        [
            (E::Comment, &self.comment),
            (E::Mention, &self.mention),
            (E::StatusChange, &self.status_change),
            (E::Assignment, &self.assignment),
            (E::QueueArrival, &self.queue_arrival),
            (E::DueSoon, &self.due_soon),
            (E::Attachment, &self.attachment),
            (E::FieldChange, &self.field_change),
        ]
        .into_iter()
        .filter_map(|(event, patch)| patch.as_ref().map(|p| (event, p)))
        .collect()
    }
}

/// Changes to the auto-watch rules: absent = unchanged, `null` = back to
/// the default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "notifications/")]
pub struct AutoWatchPatch {
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<bool>)]
    pub create: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<bool>)]
    pub assign: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<bool>)]
    pub comment: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<bool>)]
    pub mention: Patch<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<bool>)]
    pub take: Patch<bool>,
}

impl AutoWatchPatch {
    /// The entries present, with their rule.
    pub fn entries(&self) -> Vec<(AutoWatchRule, &Patch<bool>)> {
        use AutoWatchRule as R;
        [
            (R::Create, &self.create),
            (R::Assign, &self.assign),
            (R::Comment, &self.comment),
            (R::Mention, &self.mention),
            (R::Take, &self.take),
        ]
        .into_iter()
        .filter(|(_, patch)| patch.is_present())
        .collect()
    }
}

/// Changes to the digest times: absent = unchanged, `null` = the default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "notifications/")]
pub struct DigestSchedulePatch {
    /// `HH:MM` (00:00–23:59).
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub daily_time: Patch<String>,
    /// ISO weekday 1–7.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<u8>)]
    pub weekly_day: Patch<u8>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub weekly_time: Patch<String>,
}

/// `PUT /user/notifications`: only what is present changes, key by key.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "notifications/", optional_fields)]
pub struct UpdateNotificationSettingsRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events: Option<NotificationEventsPatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_watch: Option<AutoWatchPatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<DigestSchedulePatch>,
}

/// `PUT /work/workspaces/:workspace_id/notification-defaults`: key by key,
/// `null` = back to the product's.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "notifications/")]
pub struct UpdateWorkspaceNotificationDefaultsRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub events: Option<NotificationEventsPatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub auto_watch: Option<AutoWatchPatch>,
    /// Hours, [`DUE_SOON_MIN_HOURS`]–[`DUE_SOON_MAX_HOURS`].
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<u32>)]
    pub due_soon_hours: Patch<u32>,
}

/// `GET /user/notifications`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "notifications/", optional_fields)]
pub struct NotificationSettingsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
}

/// `GET /user/notifications/digests`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "notifications/", optional_fields)]
pub struct DigestListQuery {
    /// 1–[`DIGEST_LIST_MAX`], default [`DIGEST_LIST_DEFAULT`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

// ---------------------------------------------------------------------------
// Digests
// ---------------------------------------------------------------------------

/// One event in a digest: shaped like an inbox entry without its ticket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "notifications/")]
pub struct DigestEventResponse {
    /// The activity event's id.
    pub id: String,
    /// An activity verb (`ActivityAction`), a string like in the feed.
    pub action: String,
    pub event: NotificationEvent,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub detail: Option<String>,
    #[ts(type = "Record<string, unknown> | null")]
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<serde_json::Value>,
    /// RFC 3339.
    pub created_at: String,
}

/// One ticket of a digest with its events, oldest first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "notifications/")]
pub struct DigestTicketResponse {
    pub ticket_id: String,
    /// `WEB-12`, as it was when the digest was made.
    pub key: String,
    pub title: String,
    pub project_id: String,
    pub workspace_id: String,
    pub events: Vec<DigestEventResponse>,
}

/// One digest: what happened in its period, grouped by ticket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "notifications/")]
pub struct DigestResponse {
    pub id: String,
    pub cadence: DigestCadence,
    /// RFC 3339; the period is `(period_from, period_to]`.
    pub period_from: String,
    pub period_to: String,
    pub created_at: String,
    pub ticket_count: usize,
    pub event_count: usize,
    /// Events past [`DIGEST_MAX_EVENTS`] that are not listed.
    pub omitted: usize,
    /// Tickets the reader can still see, newest activity first.
    pub tickets: Vec<DigestTicketResponse>,
}

/// `GET /user/notifications/digests`: newest first, inbox digests only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "notifications/")]
pub struct DigestListResponse {
    pub digests: Vec<DigestResponse>,
}

/// The template the core mails with through notification-service
/// (`POST /api/v1/notifications/queue`).
pub const NOTIFICATION_MAIL_TEMPLATE: &str = "athene_notification";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ticket_verb_is_an_event_and_no_other_verb_is() {
        for action in ActivityAction::ALL {
            let verb = action.as_str();
            let ticket = verb.starts_with("ticket_") && !verb.starts_with("ticket_type_");
            let event = NotificationEvent::of_action(*action);
            if ticket && *action != ActivityAction::TicketDeleted {
                assert!(event.is_some(), "{action} has no event");
            } else {
                assert!(
                    event.is_none(),
                    "{action} is not about a ticket one can open"
                );
            }
        }
    }

    #[test]
    fn keys_round_trip_and_match_serde() {
        for event in NotificationEvent::ALL {
            assert_eq!(NotificationEvent::parse(event.as_str()), Some(event));
            assert_eq!(
                serde_json::to_value(event).unwrap(),
                serde_json::json!(event.as_str())
            );
        }
        for delivery in Delivery::ALL {
            assert_eq!(Delivery::parse(delivery.as_str()), Some(delivery));
            assert_eq!(
                serde_json::to_value(delivery).unwrap(),
                serde_json::json!(delivery.as_str())
            );
        }
        for rule in AutoWatchRule::ALL {
            assert_eq!(AutoWatchRule::parse(rule.as_str()), Some(rule));
            assert_eq!(
                serde_json::to_value(rule).unwrap(),
                serde_json::json!(rule.as_str())
            );
        }
    }

    #[test]
    fn the_inbox_gets_everything_at_once_by_default() {
        for event in NotificationEvent::ALL {
            assert_eq!(
                default_delivery(event, NotificationChannel::Inbox),
                Delivery::Immediate
            );
        }
        assert_eq!(
            default_delivery(NotificationEvent::Mention, NotificationChannel::Email),
            Delivery::Immediate
        );
        assert_eq!(
            default_delivery(NotificationEvent::FieldChange, NotificationChannel::Email),
            Delivery::Off
        );
    }

    #[test]
    fn a_settings_patch_tells_absent_from_null_and_refuses_unknown_keys() {
        let req: UpdateNotificationSettingsRequest = serde_json::from_value(serde_json::json!({
            "events": { "comment": { "email": null, "inbox": "daily" } },
            "auto_watch": { "comment": false },
        }))
        .unwrap();
        let events = req.events.unwrap();
        let comment = events.comment.unwrap();
        assert_eq!(comment.email, Patch::Null);
        assert_eq!(comment.inbox, Patch::Value(Delivery::Daily));
        assert_eq!(req.auto_watch.unwrap().comment, Patch::Value(false));
        for body in [
            serde_json::json!({ "events": { "comments": {} } }),
            serde_json::json!({ "events": { "comment": { "push": "daily" } } }),
            serde_json::json!({ "events": { "comment": { "inbox": "hourly" } } }),
            serde_json::json!({ "auto_watch": { "watch": true } }),
            serde_json::json!({ "digest": { "daily_time": "08:00", "tz": "x" } }),
        ] {
            assert!(
                serde_json::from_value::<UpdateNotificationSettingsRequest>(body.clone()).is_err(),
                "{body}"
            );
        }
    }
}
