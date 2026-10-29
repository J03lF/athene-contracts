//! Integrations of a workspace (W13-W): outgoing webhooks, Slack and
//! Microsoft Teams.
//!
//! An **integration** sends chosen events of a workspace to one address:
//!
//! - [`IntegrationKind::Webhook`]: any `https` endpoint. The body is JSON
//!   ([`WEBHOOK_PAYLOAD_VERSION`]), signed with the integration's own secret
//!   (HMAC-SHA256, [`SIGNATURE_HEADER`]); the secret is shown once, when it
//!   is minted (create, rotate).
//! - [`IntegrationKind::Slack`]: a Slack *incoming webhook* URL
//!   (`https://hooks.slack.com/services/…`); the message is Block Kit.
//! - [`IntegrationKind::Teams`]: a Microsoft Teams incoming webhook or a
//!   Workflows ("Power Automate") webhook URL; the message is an Adaptive
//!   Card in the `message` envelope both accept.
//!
//! Every change of the workspace that an integration listens to is written
//! to an outbox in the same transaction as the change; a background worker
//! in the core delivers it (never the request), retries with exponential
//! backoff and keeps the last [`INTEGRATION_DELIVERY_LOG_SIZE`] deliveries
//! per integration. After [`INTEGRATION_MAX_ATTEMPTS`] failed attempts a
//! delivery is *dead*; [`INTEGRATION_PAUSE_AFTER_DEAD`] dead deliveries in
//! a row pause the integration ([`IntegrationState::Failing`]) and the
//! workspace's admins get a mail.
//!
//! Addresses are checked before anything is sent: `https` only (dev
//! installations also take `http`), no user info, and every address the
//! host name resolves to must be public — loopback, private, link-local,
//! carrier-grade NAT, metadata (`169.254.169.254`, `fd00:ec2::254`) and the
//! other special ranges of IPv4 and IPv6 are refused, also for an address
//! written as a literal. Redirects are not followed. The URL (it carries
//! the secret of a Slack or Teams hook) and the signing secret are stored
//! encrypted; responses show the URL masked ([`IntegrationResponse::url_display`]).
//!
//! Routes (all `workspace.settings.edit`: the workspace's owners and
//! admins; every response is `{ "data": … }`, request bodies refuse
//! unknown fields):
//!
//! - `GET    /work/workspaces/:workspace_id/integrations`
//! - `POST   /work/workspaces/:workspace_id/integrations`
//! - `PATCH  /work/workspaces/:workspace_id/integrations/:integration_id`
//! - `DELETE /work/workspaces/:workspace_id/integrations/:integration_id`
//! - `POST   /work/workspaces/:workspace_id/integrations/:integration_id/secret`
//!   (webhooks: a new signing secret, shown once)
//! - `POST   /work/workspaces/:workspace_id/integrations/:integration_id/test`
//!   ("Test senden": a test delivery, sent by the worker within seconds)
//! - `GET    /work/workspaces/:workspace_id/integrations/:integration_id/deliveries`
//! - `POST   /work/workspaces/:workspace_id/integrations/:integration_id/deliveries/:delivery_id/redeliver`
//!
//! Refusals: `400 ATH-VAL-001` with `details.reason` one of
//! [`UrlRefusal`]'s keys (and `details.field`); `409 ATH-RES-002` with
//! `details.reason = "limit"` past [`INTEGRATION_MAX_PER_WORKSPACE`];
//! `409 ATH-RES-003` with `details.current` when `version` is stale;
//! `429 ATH-RATE-001` past [`INTEGRATION_MANUAL_SENDS_PER_MINUTE`] tests
//! and redeliveries per workspace; `403 ATH-FEAT-001` with
//! `details.reason = "integrations_not_configured"` when the server has no
//! key to store secrets with.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::activity::ActivityAction;

/// The most integrations (all kinds together) a workspace may hold.
pub const INTEGRATION_MAX_PER_WORKSPACE: usize = 20;
/// An integration's name, after trimming (one line).
pub const INTEGRATION_NAME_MAX_CHARS: usize = 80;
/// The longest address.
pub const INTEGRATION_URL_MAX_CHARS: usize = 2048;
/// The most projects an integration may be limited to.
pub const INTEGRATION_MAX_PROJECTS: usize = 50;
/// Deliveries kept per integration (the log).
pub const INTEGRATION_DELIVERY_LOG_SIZE: usize = 100;
/// Attempts of one delivery before it is dead.
pub const INTEGRATION_MAX_ATTEMPTS: u32 = 8;
/// Dead deliveries in a row that pause an integration.
pub const INTEGRATION_PAUSE_AFTER_DEAD: u32 = 3;
/// Tests and redeliveries per workspace and minute.
pub const INTEGRATION_MANUAL_SENDS_PER_MINUTE: u32 = 10;
/// One attempt's whole time (connect, TLS, request, answer), in seconds.
pub const INTEGRATION_TIMEOUT_SECONDS: u64 = 10;
/// The most of an answer that is read, in bytes; the log keeps less.
pub const INTEGRATION_RESPONSE_MAX_BYTES: usize = 64 * 1024;

/// The version of the webhook body (`"version"` in it).
pub const WEBHOOK_PAYLOAD_VERSION: &str = "2026-10-29";
/// `t=<unix seconds>,v1=<hex HMAC-SHA256 of "<t>.<raw body>">`.
pub const SIGNATURE_HEADER: &str = "X-Athenos-Signature";
/// The unix seconds the signature covers (also inside the signature).
pub const TIMESTAMP_HEADER: &str = "X-Athenos-Timestamp";
/// The event's wire name ([`IntegrationEvent::as_str`] or `integration.test`).
pub const EVENT_HEADER: &str = "X-Athenos-Event";
/// The delivery's id; a redelivery has a new one, the body's `id` stays.
pub const DELIVERY_HEADER: &str = "X-Athenos-Delivery";
/// How old a signed timestamp a receiver should still accept, in seconds.
pub const SIGNATURE_TOLERANCE_SECONDS: i64 = 300;
/// The prefix of a signing secret (`whsec_` and 32 letters and digits).
pub const SIGNING_SECRET_PREFIX: &str = "whsec_";
/// The event name of a test delivery.
pub const TEST_EVENT: &str = "integration.test";
/// `details.reason` of the 403 when the server cannot store secrets.
pub const NOT_CONFIGURED_REASON: &str = "integrations_not_configured";
/// `details.reason` of the 409 past [`INTEGRATION_MAX_PER_WORKSPACE`].
pub const LIMIT_REASON: &str = "limit";

/// Where an integration sends to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "integrations/")]
pub enum IntegrationKind {
    Webhook,
    Slack,
    Teams,
}

impl IntegrationKind {
    pub const ALL: [Self; 3] = [Self::Webhook, Self::Slack, Self::Teams];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Webhook => "webhook",
            Self::Slack => "slack",
            Self::Teams => "teams",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == value)
    }

    /// Only webhooks sign their bodies (Slack and Teams hooks carry their
    /// secret in the URL).
    pub const fn signs(self) -> bool {
        matches!(self, Self::Webhook)
    }
}

/// What an integration can be told about. The wire names are the
/// webhook's `type` and the [`EVENT_HEADER`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS,
)]
#[ts(export_to = "integrations/")]
pub enum IntegrationEvent {
    #[serde(rename = "ticket.created")]
    TicketCreated,
    /// Title, description, priority, type, due date, labels, custom
    /// fields, sprint or project changed.
    #[serde(rename = "ticket.updated")]
    TicketUpdated,
    #[serde(rename = "ticket.status_changed")]
    TicketStatusChanged,
    #[serde(rename = "ticket.assigned")]
    TicketAssigned,
    #[serde(rename = "ticket.commented")]
    TicketCommented,
    #[serde(rename = "ticket.deleted")]
    TicketDeleted,
    #[serde(rename = "sprint.started")]
    SprintStarted,
    #[serde(rename = "sprint.completed")]
    SprintCompleted,
    /// A release was marked as released.
    #[serde(rename = "release.published")]
    ReleasePublished,
}

impl IntegrationEvent {
    /// Every event, in the order the settings show them.
    pub const ALL: [Self; 9] = [
        Self::TicketCreated,
        Self::TicketUpdated,
        Self::TicketStatusChanged,
        Self::TicketAssigned,
        Self::TicketCommented,
        Self::TicketDeleted,
        Self::SprintStarted,
        Self::SprintCompleted,
        Self::ReleasePublished,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TicketCreated => "ticket.created",
            Self::TicketUpdated => "ticket.updated",
            Self::TicketStatusChanged => "ticket.status_changed",
            Self::TicketAssigned => "ticket.assigned",
            Self::TicketCommented => "ticket.commented",
            Self::TicketDeleted => "ticket.deleted",
            Self::SprintStarted => "sprint.started",
            Self::SprintCompleted => "sprint.completed",
            Self::ReleasePublished => "release.published",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.as_str() == value)
    }

    /// The event an activity verb is. Exhaustive: a new verb has to say
    /// whether an integration hears about it. The outbox trigger takes the
    /// verbs from the integration's row ([`Self::actions`] of its events),
    /// so this match is the only list.
    pub const fn of_action(action: ActivityAction) -> Option<Self> {
        use ActivityAction as A;
        match action {
            A::TicketCreated => Some(Self::TicketCreated),
            A::TicketUpdated
            | A::TicketCustomValuesUpdated
            | A::TicketLabeled
            | A::TicketMovedProject
            | A::TicketSprintAssigned
            | A::TicketSprintUnassigned => Some(Self::TicketUpdated),
            A::TicketStatusChanged => Some(Self::TicketStatusChanged),
            A::TicketAssigned => Some(Self::TicketAssigned),
            A::TicketCommented => Some(Self::TicketCommented),
            A::TicketDeleted => Some(Self::TicketDeleted),
            A::SprintStarted => Some(Self::SprintStarted),
            A::SprintCompleted => Some(Self::SprintCompleted),
            A::MilestoneReleased => Some(Self::ReleasePublished),
            A::TicketTriaged
            | A::TicketCommentEdited
            | A::TicketMentioned
            | A::TicketLinked
            | A::TicketWorkLogged
            | A::TicketDueFollowedSprint
            | A::TicketAttachmentAdded
            | A::TicketAttachmentRemoved
            | A::TicketAttachmentRestored
            | A::TicketAttachmentPurged
            | A::TicketAttachmentRenamed
            | A::TicketReleaseNoteUpdated
            | A::TicketDueSoon
            | A::ProjectUpdated
            | A::ProjectCustomValuesUpdated
            | A::SprintCreated
            | A::SprintUpdated
            | A::SprintRescheduled
            | A::SprintDeleted
            | A::SprintAssignedTickets
            | A::SprintUnassignedTickets
            | A::MilestoneCreated
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
            | A::WorkflowStatusCreated
            | A::WorkflowStatusUpdated
            | A::WorkflowStatusArchived
            | A::WorkflowStatusDeleted
            | A::EventSeriesCreated
            | A::EventSeriesUpdated
            | A::EventSeriesSplit
            | A::EventSeriesEnded
            | A::EventSeriesDeleted
            | A::EventOccurrenceUpdated
            | A::EventOccurrenceCancelled => None,
        }
    }

    /// The activity verbs that are this event.
    pub fn actions(self) -> Vec<ActivityAction> {
        ActivityAction::ALL
            .iter()
            .copied()
            .filter(|a| Self::of_action(*a) == Some(self))
            .collect()
    }

    /// Whether the event is about a ticket (the project filter applies to
    /// its project; sprints and releases have one too).
    pub const fn is_ticket(self) -> bool {
        matches!(
            self,
            Self::TicketCreated
                | Self::TicketUpdated
                | Self::TicketStatusChanged
                | Self::TicketAssigned
                | Self::TicketCommented
                | Self::TicketDeleted
        )
    }
}

/// Whether an integration sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "integrations/")]
pub enum IntegrationState {
    Active,
    /// Switched off by an admin.
    Paused,
    /// Paused by the server after [`INTEGRATION_PAUSE_AFTER_DEAD`] dead
    /// deliveries in a row; switching it on again clears it.
    Failing,
}

impl IntegrationState {
    pub const ALL: [Self; 3] = [Self::Active, Self::Paused, Self::Failing];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Failing => "failing",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.as_str() == value)
    }
}

/// The language of a Slack or Teams message (a webhook body has none).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "integrations/")]
pub enum IntegrationLanguage {
    De,
    En,
}

impl IntegrationLanguage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::De => "de",
            Self::En => "en",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "de" => Some(Self::De),
            "en" => Some(Self::En),
            _ => None,
        }
    }
}

/// Why an address is refused (`details.reason` of the 400).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "integrations/")]
pub enum UrlRefusal {
    /// Not a URL, or longer than [`INTEGRATION_URL_MAX_CHARS`].
    InvalidUrl,
    /// Not `https` (a dev installation also takes `http`).
    HttpsRequired,
    /// User name or password in the URL.
    Credentials,
    /// A host that is or resolves to a loopback, private, link-local or
    /// otherwise non-public address, or a local name (`localhost`, `.local`,
    /// `.internal`).
    PrivateAddress,
    /// Not a Slack incoming-webhook address.
    NotSlack,
    /// Not a Teams or Workflows webhook address.
    NotTeams,
}

impl UrlRefusal {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidUrl => "invalid_url",
            Self::HttpsRequired => "https_required",
            Self::Credentials => "credentials",
            Self::PrivateAddress => "private_address",
            Self::NotSlack => "not_slack",
            Self::NotTeams => "not_teams",
        }
    }
}

/// A delivery's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "integrations/")]
pub enum DeliveryStatus {
    /// Waiting for its first attempt.
    Pending,
    /// Failed, the next attempt is at `next_attempt_at`.
    Retrying,
    Succeeded,
    /// Given up: [`INTEGRATION_MAX_ATTEMPTS`] attempts, an answer that
    /// retrying cannot change (a `4xx` but `408`/`429`), or the
    /// integration was deleted meanwhile.
    Dead,
}

impl DeliveryStatus {
    pub const ALL: [Self; 4] = [Self::Pending, Self::Retrying, Self::Succeeded, Self::Dead];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Retrying => "retrying",
            Self::Succeeded => "succeeded",
            Self::Dead => "dead",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.as_str() == value)
    }
}

/// What went wrong with an attempt besides the status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "integrations/")]
pub enum DeliveryError {
    /// The answer was not `2xx` (the code is `response_status`).
    HttpStatus,
    /// No answer within [`INTEGRATION_TIMEOUT_SECONDS`].
    Timeout,
    /// No connection (DNS, refused, reset, TLS).
    Connect,
    /// The host is or resolved to a non-public address.
    BlockedAddress,
    /// A `3xx`: redirects are not followed.
    Redirect,
    /// The answer was larger than [`INTEGRATION_RESPONSE_MAX_BYTES`].
    ResponseTooLarge,
    /// The stored address or secret could not be read (key changed).
    SecretUnavailable,
    /// The workspace's send budget per minute was used up; the attempt
    /// waits, it is not counted.
    RateLimited,
}

impl DeliveryError {
    pub const ALL: [Self; 8] = [
        Self::HttpStatus,
        Self::Timeout,
        Self::Connect,
        Self::BlockedAddress,
        Self::Redirect,
        Self::ResponseTooLarge,
        Self::SecretUnavailable,
        Self::RateLimited,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HttpStatus => "http_status",
            Self::Timeout => "timeout",
            Self::Connect => "connect",
            Self::BlockedAddress => "blocked_address",
            Self::Redirect => "redirect",
            Self::ResponseTooLarge => "response_too_large",
            Self::SecretUnavailable => "secret_unavailable",
            Self::RateLimited => "rate_limited",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.as_str() == value)
    }
}

/// An integration as the settings show it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct IntegrationResponse {
    pub id: String,
    pub workspace_id: String,
    pub kind: IntegrationKind,
    pub name: String,
    /// The address with its path and query masked
    /// (`https://hooks.slack.com/services/T0…/•••`): the full URL is never
    /// read back.
    pub url_display: String,
    /// In [`IntegrationEvent::ALL`] order.
    pub events: Vec<IntegrationEvent>,
    /// Empty: every project of the workspace.
    pub project_ids: Vec<String>,
    pub language: IntegrationLanguage,
    pub state: IntegrationState,
    /// Webhooks: the signing secret's last four characters
    /// (`whsec_…3fQa`); `null` for Slack and Teams.
    pub secret_hint: Option<String>,
    /// Dead deliveries since the last success.
    pub failures_in_row: i32,
    pub last_success_at: Option<String>,
    pub last_failure_at: Option<String>,
    /// The last failed attempt's error and status code, for the card.
    pub last_error: Option<DeliveryError>,
    pub last_status: Option<i32>,
    /// When the server paused it ([`IntegrationState::Failing`]).
    pub paused_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    /// For `PATCH` (`expected_version`).
    pub version: i64,
}

/// `GET /work/workspaces/:workspace_id/integrations`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct WorkspaceIntegrationsResponse {
    /// Newest first within each kind; the web groups by kind.
    pub integrations: Vec<IntegrationResponse>,
    /// The server can store secrets (a key is set). Without it nothing can
    /// be created; the page says so.
    pub configured: bool,
    /// This server takes `http` addresses too (a dev installation).
    pub allows_http: bool,
    /// This server sends to loopback and private addresses too
    /// (`ATHENOS_INTEGRATIONS_ALLOW_PRIVATE`, a dev installation; a
    /// production core refuses to start with it). The web then leaves its
    /// own check of local names and private addresses to the server.
    /// Absent from an older core: `false`.
    #[serde(default)]
    pub allows_private: bool,
}

/// `POST /work/workspaces/:workspace_id/integrations`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "integrations/")]
pub struct CreateIntegrationRequest {
    pub kind: IntegrationKind,
    /// 1–[`INTEGRATION_NAME_MAX_CHARS`] characters, trimmed, one line.
    pub name: String,
    /// See the module docs for what is refused.
    pub url: String,
    /// At least one; repeats are dropped.
    pub events: Vec<IntegrationEvent>,
    /// Absent or empty: every project.
    #[serde(default)]
    #[ts(optional)]
    pub project_ids: Option<Vec<String>>,
    /// Absent: `de`.
    #[serde(default)]
    #[ts(optional)]
    pub language: Option<IntegrationLanguage>,
}

/// `201`: the integration and, for a webhook, its signing secret — the only
/// time it is shown.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct IntegrationCreatedResponse {
    pub integration: IntegrationResponse,
    pub signing_secret: Option<String>,
}

/// `PATCH /work/workspaces/:workspace_id/integrations/:integration_id`:
/// every field optional, `expected_version` required (`409 ATH-RES-003`
/// with `details.current` when it moved).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "integrations/")]
pub struct UpdateIntegrationRequest {
    #[serde(default)]
    #[ts(optional)]
    pub name: Option<String>,
    /// A new address (the old one is never sent back to edit).
    #[serde(default)]
    #[ts(optional)]
    pub url: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub events: Option<Vec<IntegrationEvent>>,
    /// Empty: every project.
    #[serde(default)]
    #[ts(optional)]
    pub project_ids: Option<Vec<String>>,
    #[serde(default)]
    #[ts(optional)]
    pub language: Option<IntegrationLanguage>,
    /// `false` pauses it; `true` switches it on again (also after the
    /// server paused it) and clears `failures_in_row`.
    #[serde(default)]
    #[ts(optional)]
    pub enabled: Option<bool>,
    pub expected_version: i64,
}

/// `POST …/secret`: the new signing secret, shown once. The old one stops
/// signing at once.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct IntegrationSecretResponse {
    pub integration: IntegrationResponse,
    pub signing_secret: String,
}

/// `DELETE`: the integration and its log are gone; queued deliveries are
/// dropped.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct DeleteIntegrationResponse {
    pub id: String,
}

/// One delivery of the log.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct DeliveryResponse {
    pub id: String,
    pub integration_id: String,
    /// [`IntegrationEvent::as_str`] or [`TEST_EVENT`].
    pub event: String,
    /// What it was about, for the log line (`WEB-42 Login bricht ab`,
    /// a sprint's or release's name); `null` for a test.
    pub subject: Option<String>,
    pub status: DeliveryStatus,
    /// Attempts made so far.
    pub attempts: i32,
    /// The last attempt's answer code, if there was an answer.
    pub response_status: Option<i32>,
    /// The last attempt's time to the answer, in milliseconds.
    pub latency_ms: Option<i64>,
    pub error: Option<DeliveryError>,
    /// The first 300 characters of the last answer's body, as text.
    pub response_excerpt: Option<String>,
    pub next_attempt_at: Option<String>,
    pub last_attempt_at: Option<String>,
    pub created_at: String,
    /// A redelivery's original.
    pub redelivery_of: Option<String>,
}

/// `GET …/deliveries`: the last [`INTEGRATION_DELIVERY_LOG_SIZE`], newest
/// first.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct IntegrationDeliveriesResponse {
    pub deliveries: Vec<DeliveryResponse>,
}

/// `POST …/test` and `…/redeliver` (`202`): the queued delivery; read the
/// log again for its outcome.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct DeliveryQueuedResponse {
    pub delivery: DeliveryResponse,
}

/// The body of a webhook delivery ([`IntegrationKind::Webhook`]), version
/// [`WEBHOOK_PAYLOAD_VERSION`]. Exactly one of `ticket`, `sprint` and
/// `release` is set for an event, none for a test. Names and titles are
/// what the event is about; no e-mail address or other personal data
/// beyond the acting person's and the assignee's display name travels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct WebhookPayload {
    /// The event's id: the same for a redelivery (deduplicate on it).
    pub id: String,
    pub version: String,
    /// [`IntegrationEvent::as_str`] or [`TEST_EVENT`].
    #[serde(rename = "type")]
    pub event_type: String,
    /// When it happened (RFC 3339).
    pub created_at: String,
    pub workspace: PayloadRef,
    /// `null`: the system or an automation.
    pub actor: Option<PayloadPerson>,
    pub ticket: Option<PayloadTicket>,
    pub sprint: Option<PayloadSprint>,
    pub release: Option<PayloadRelease>,
    /// What changed: a status change's `status`, an assignment's
    /// `assignee`, an update's `fields`.
    pub changes: Option<PayloadChanges>,
    /// `ticket.commented`: the comment.
    pub comment: Option<PayloadComment>,
}

/// An id with its name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct PayloadRef {
    pub id: String,
    pub name: String,
}

/// A person: their id and display name only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct PayloadPerson {
    pub id: String,
    pub name: String,
}

/// A project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct PayloadProject {
    pub id: String,
    pub key: String,
    pub name: String,
}

/// A status: its key and the workspace's name for it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct PayloadStatus {
    pub key: String,
    pub name: String,
}

/// A ticket as it is after the event (for `ticket.deleted`: as it was;
/// then only `id`, `key`, `title` and `project` are known).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct PayloadTicket {
    pub id: String,
    /// `WEB-42`.
    pub key: String,
    pub title: String,
    /// The ticket in the web app; `null` for a deleted one or when the
    /// server has no public address.
    pub url: Option<String>,
    pub status: Option<PayloadStatus>,
    /// `low` | `medium` | `high` | `critical`.
    pub priority: Option<String>,
    pub project: Option<PayloadProject>,
    pub assignee: Option<PayloadPerson>,
    /// The description's first 280 characters as plain text.
    pub excerpt: Option<String>,
}

/// A sprint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct PayloadSprint {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
    pub project: Option<PayloadProject>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
}

/// A release.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct PayloadRelease {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
    pub project: Option<PayloadProject>,
}

/// A value before and after.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct PayloadChange {
    pub from: Option<String>,
    pub to: Option<String>,
}

/// What an event changed.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct PayloadChanges {
    /// Status keys.
    pub status: Option<PayloadChange>,
    /// Assignee ids (`to` is `null` when it was cleared).
    pub assignee: Option<PayloadChange>,
    /// The fields an update changed (`title`, `priority`, `labels`, …).
    pub fields: Option<Vec<String>>,
}

/// A comment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "integrations/")]
pub struct PayloadComment {
    pub id: Option<String>,
    /// Its first 280 characters as plain text.
    pub excerpt: Option<String>,
}

/// How long an excerpt may be, in characters.
pub const PAYLOAD_EXCERPT_MAX_CHARS: usize = 280;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_event_has_a_verb_and_round_trips() {
        for event in IntegrationEvent::ALL {
            assert!(!event.actions().is_empty(), "{event:?} hears no verb");
            assert_eq!(IntegrationEvent::parse(event.as_str()), Some(event));
            let wire = serde_json::to_value(event).unwrap();
            assert_eq!(wire, serde_json::json!(event.as_str()));
        }
        assert_eq!(
            IntegrationEvent::of_action(ActivityAction::MilestoneReleased),
            Some(IntegrationEvent::ReleasePublished)
        );
        assert_eq!(
            IntegrationEvent::of_action(ActivityAction::TicketDueSoon),
            None
        );
    }

    #[test]
    fn the_enums_wire_names_are_their_keys() {
        for kind in IntegrationKind::ALL {
            assert_eq!(
                serde_json::to_value(kind).unwrap(),
                serde_json::json!(kind.as_str())
            );
        }
        for state in IntegrationState::ALL {
            assert_eq!(
                serde_json::to_value(state).unwrap(),
                serde_json::json!(state.as_str())
            );
        }
        for status in DeliveryStatus::ALL {
            assert_eq!(
                serde_json::to_value(status).unwrap(),
                serde_json::json!(status.as_str())
            );
        }
        for error in DeliveryError::ALL {
            assert_eq!(
                serde_json::to_value(error).unwrap(),
                serde_json::json!(error.as_str())
            );
        }
    }

    #[test]
    fn requests_refuse_unknown_fields() {
        let bad = serde_json::json!({
            "kind": "slack", "name": "x", "url": "https://hooks.slack.com/services/a",
            "events": ["ticket.created"], "secret": "nope"
        });
        assert!(serde_json::from_value::<CreateIntegrationRequest>(bad).is_err());
        let ok = serde_json::json!({ "expected_version": 1, "enabled": false });
        assert!(serde_json::from_value::<UpdateIntegrationRequest>(ok).is_ok());
    }
}
