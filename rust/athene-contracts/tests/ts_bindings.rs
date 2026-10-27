//! The generated TypeScript in `athene-contracts/src/generated/` must match
//! what the Rust contract types produce now.
//!
//! Check (CI, and part of every `cargo test`):
//!     cargo test -p athene-contracts --test ts_bindings
//! Regenerate after changing a contract type:
//!     ATHENE_CONTRACTS_REGENERATE=1 cargo test -p athene-contracts --test ts_bindings
//! (`npm run generate` / `npm run check:generated` in the package root wrap
//! both.)
//!
//! Types are listed here explicitly; their dependencies (e.g. the
//! `RegistrationMode` of the settings) are exported with them. 64-bit
//! integers become `number`: the counts and ids they carry stay far below
//! 2^53, and the JSON on the wire is a plain number. A `Patch<T>` field
//! becomes `key?: T | null` (see `athene_contracts::patch`).
//!
//! Besides the types, `policy/policy-matrix.json` is rendered here: every
//! verdict of `athene_contracts::policy::allows` for the matrix subjects,
//! so a rule change shows up as a diff of that file too; and
//! `workspace/invite-constants.ts`, the invitation limits as values.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use athene_contracts::{
    activity, admin, athene_ai, attachments, auth, calendar, common, dashboard, docs, edit,
    field_assignments, field_definitions, listing, overrides, policy, queues, roles, search, teams,
    ticket_check, ticket_types, user, work, workspace,
};
use ts_rs::{Config, TS};

const REGENERATE_ENV: &str = "ATHENE_CONTRACTS_REGENERATE";

fn committed_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/generated")
}

/// Writes the bindings of every listed type (and its dependencies) to `out`.
fn render(out: &Path) {
    let cfg = Config::new().with_out_dir(out).with_large_int("number");
    macro_rules! export {
        ($($ty:ty),* $(,)?) => {
            $( <$ty as TS>::export_all(&cfg).expect(stringify!($ty)); )*
        };
    }
    export!(
        admin::FeatureFlagRequest,
        admin::UpdateFeatureFlagRequest,
        admin::ToggleRequest,
        admin::FeatureFlagResponse,
        admin::CreateApiKeyRequest,
        admin::ApiKeyResponse,
        admin::ApiKeyCreatedResponse,
        admin::ApiKeyRevokedResponse,
        admin::MaintenanceRequest,
        admin::UpdateSettingsRequest,
        admin::SettingsResponse,
        admin::MaintenanceStatusResponse,
        admin::TerminateSessionsResponse,
        admin::AuditEntryResponse,
        admin::CacheRefreshResponse,
        admin::OverviewCounts,
        admin::OverviewResponse,
        // work: vocabulary
        work::TicketStatus,
        work::TicketPriority,
        work::ProjectHealth,
        work::ProjectStatus,
        work::SprintStatus,
        work::MilestoneStatus,
        work::LinkRelation,
        work::InboxReason,
        // work: query strings
        work::ListTicketsQuery,
        work::ListProjectsQuery,
        work::PerUserListQuery,
        work::WorkScopeQuery,
        work::ProjectReportQuery,
        // paged ticket lists (3.9)
        listing::Page<work::TicketResponse>,
        listing::Page<listing::TicketSummary>,
        listing::PageMeta,
        listing::TicketSort,
        listing::SortOrder,
        listing::TicketFields,
        listing::DueSource,
        listing::EffectiveDue,
        listing::TicketSummary,
        listing::WorkspaceSprintsQuery,
        listing::CustomFieldOp,
        listing::CustomFieldFilter,
        // work: projects and tickets
        work::ProjectResponse,
        work::CreateProjectRequest,
        work::UpdateProjectRequest,
        work::ProjectExpected,
        work::UpdateCustomValuesRequest,
        work::TicketResponse,
        work::CreateTicketRequest,
        work::UpdateTicketRequest,
        work::TicketExpected,
        work::TicketEdit,
        work::BulkUpdateTicketsRequest,
        work::BulkUpdateFailure,
        work::BulkUpdateConflict,
        work::BulkUpdateTicketsResponse,
        work::BulkSetFieldRequest,
        work::MoveTicketProjectRequest,
        work::MoveTicketProjectResponse,
        work::TicketProjectMove,
        work::DeletedTicketResponse,
        work::UpdateTicketStatusRequest,
        work::MoveTicketRequest,
        work::AssignTicketRequest,
        work::TriageTicketRequest,
        work::WorkStatsResponse,
        work::WorkActivityEntryResponse,
        work::InboxEntryResponse,
        // work: comments, links, watchers
        work::TicketCommentResponse,
        work::AddTicketCommentRequest,
        work::TicketLinkResponse,
        work::CreateTicketLinkRequest,
        work::TicketLinkDeletedResponse,
        work::TicketWatcherResponse,
        work::TicketWatcherListResponse,
        work::WatchStateResponse,
        // work: sprints and releases
        work::SprintResponse,
        work::SprintCompletionStats,
        work::CreateSprintRequest,
        work::UpdateSprintRequest,
        work::SprintExpected,
        work::CompleteSprintRequest,
        work::AssignTicketsRequest,
        work::SprintBurnDownResponse,
        work::MilestoneProgress,
        work::MilestoneResponse,
        work::CreateMilestoneRequest,
        work::UpdateMilestoneRequest,
        // work: labels, work logs, reports
        work::LabelResponse,
        work::CreateLabelRequest,
        work::UpdateLabelRequest,
        work::LabelAssignmentResponse,
        work::SetLabelsRequest,
        work::WorkLogResponse,
        work::WorkLogListResponse,
        work::AddWorkLogRequest,
        work::BurndownPointResponse,
        work::VelocityEntryResponse,
        work::FlowPointResponse,
        work::StatusShareResponse,
        work::ReportSprintResponse,
        work::ProjectReportResponse,
        // edits: conflict handling (step 3.7)
        edit::SetOps<String>,
        edit::TextHash,
        edit::EditConflictDetails<work::TicketResponse>,
        edit::FieldConflict,
        edit::ChangedBy,
        // auth (the rest of auth is still hand-written)
        auth::FeatureFlagState,
        auth::StreamTicket,
        // common
        common::ApiErrorCode,
        // user
        user::Theme,
        user::Density,
        user::DashboardWidgetLayout,
        user::SavedView,
        user::InboxState,
        user::UserSettingsResponse,
        user::UpdateUserSettingsRequest,
        user::UserProfileResponse,
        user::UpdateProfileRequest,
        // activity and dashboard
        attachments::AttachmentKind,
        attachments::AttachmentStatus,
        attachments::AttachmentResponse,
        attachments::ListAttachmentsQuery,
        attachments::AttachmentListResponse,
        attachments::AttachmentContentQuery,
        attachments::AttachmentExpected,
        attachments::RenameAttachmentRequest,
        activity::ActivityEventResponse,
        activity::ActivityAction,
        dashboard::KpiTile,
        dashboard::KpisResponse,
        dashboard::FocusTicket,
        dashboard::FocusTab,
        dashboard::OnTrackState,
        dashboard::OnTrackResponse,
        dashboard::DashboardOverviewResponse,
        // workspace
        workspace::WorkspaceResponse,
        workspace::WorkspaceSettings,
        workspace::WorkspaceSettingsPatch,
        workspace::StatusLabels,
        workspace::ComposerProperty,
        workspace::ComposerPropertiesSetting,
        workspace::DefaultAssignee,
        workspace::WeekStart,
        workspace::UpdateWorkspaceRequest,
        workspace::CreateWorkspaceRequest,
        workspace::InviteDetailsResponse,
        workspace::JoinInviteResponse,
        workspace::WorkspaceMemberRole,
        workspace::WorkspaceMemberResponse,
        workspace::InvitationStatus,
        workspace::InvitationResponse,
        workspace::CreateInviteRequest,
        workspace::UpdateMemberRoleRequest,
        // field definitions
        field_definitions::AppliesTo,
        field_definitions::FieldType,
        field_definitions::SelectChoice,
        field_definitions::SelectFieldOptions,
        field_definitions::NumberFieldOptions,
        field_definitions::TextFieldOptions,
        field_definitions::FieldOptions,
        field_definitions::FieldDefinitionResponse,
        field_definitions::CreateFieldDefinitionRequest,
        field_definitions::UpdateFieldDefinitionRequest,
        field_definitions::ReorderFieldDefinitionsRequest,
        field_definitions::VersionFieldOptions,
        field_definitions::UserFieldOptions,
        field_definitions::ListFieldDefinitionsQuery,
        field_definitions::ChoiceOps,
        field_definitions::FieldDefinitionExpected,
        edit::EditConflictDetails<field_definitions::FieldDefinitionResponse>,
        // field rules, fields of one ticket, suggestions (custom fields CF1)
        field_assignments::RequiredOn,
        field_assignments::FieldAssignmentResponse,
        field_assignments::FieldRuleDraft,
        field_assignments::CreateFieldAssignmentRequest,
        field_assignments::UpdateFieldAssignmentRequest,
        field_assignments::FieldAssignmentExpected,
        edit::EditConflictDetails<field_assignments::FieldAssignmentResponse>,
        field_assignments::AttachTicketFieldRequest,
        field_assignments::DetachTicketFieldQuery,
        field_assignments::FieldSuggestionStatus,
        field_assignments::FieldDraft,
        field_assignments::FieldSuggestionResponse,
        field_assignments::CreateFieldSuggestionRequest,
        field_assignments::ListFieldSuggestionsQuery,
        field_assignments::AcceptFieldSuggestionRequest,
        field_assignments::AcceptFieldSuggestionResponse,
        field_assignments::FieldContext,
        field_assignments::ResolutionDefinition,
        field_assignments::ResolutionRule,
        field_assignments::FieldOrigin,
        field_assignments::ResolvedField,
        // teams
        teams::TeamRole,
        teams::TeamMemberResponse,
        teams::TeamResponse,
        teams::TeamMemberInput,
        teams::CreateTeamRequest,
        teams::UpdateTeamRequest,
        teams::AddTeamMembersRequest,
        teams::UpdateTeamMemberRequest,
        teams::TeamListQuery,
        // ticket types
        ticket_types::TicketTypeIcon,
        ticket_types::TicketTypeColor,
        ticket_types::TicketTypeResponse,
        ticket_types::CreateTicketTypeRequest,
        ticket_types::UpdateTicketTypeRequest,
        ticket_types::ReorderTicketTypesRequest,
        ticket_types::ListTicketTypesQuery,
        // docs
        docs::DocCollectionResponse,
        docs::CreateDocCollectionRequest,
        docs::UpdateDocCollectionRequest,
        docs::DocSummaryResponse,
        docs::DocListMeta,
        docs::DocListResponse,
        docs::DocResponse,
        docs::CreateDocRequest,
        docs::UpdateDocRequest,
        docs::DocExpected,
        docs::DocEdit,
        docs::DocListQuery,
        docs::DocSort,
        // policy (3.8a)
        policy::Action,
        policy::ActionKind,
        policy::ActionScope,
        policy::Fact,
        policy::Standing,
        policy::Permission,
        policy::PermissionGroup,
        policy::PermissionSource,
        policy::EffectivePermission,
        policy::PermissionStep,
        // permission overrides (custom roles, phase B)
        overrides::OverridePlace,
        overrides::OverrideTarget,
        overrides::OverrideState,
        overrides::OverrideLevel,
        overrides::TeamOverrideMasks,
        overrides::PlaceQuery,
        overrides::PermissionOverrideResponse,
        overrides::WorkspaceOverridesResponse,
        overrides::SetOverrideRequest,
        overrides::SetOverrideResponse,
        policy::RolePreset,
        policy::CapabilitiesRole,
        policy::CapabilitiesResponse,
        // workspace roles (custom roles, phase A)
        roles::WorkspaceRoleResponse,
        roles::RolePresetResponse,
        roles::RolesNotice,
        roles::WorkspaceRolesResponse,
        roles::CreateRoleRequest,
        roles::CreatePresetRolesRequest,
        roles::UpdateRoleRequest,
        roles::ReorderRolesRequest,
        roles::DeleteRoleRequest,
        roles::DeleteRoleResponse,
        roles::EffectiveRoleResponse,
        roles::AnswerRolesNoticeRequest,
        // custom roles C1: queues and hand-offs
        queues::QueueTargetKind,
        queues::QueueResponse,
        queues::WorkspaceQueuesResponse,
        queues::CreateQueueRequest,
        queues::QueueExpected,
        queues::UpdateQueueRequest,
        queues::DeleteQueueResponse,
        queues::QueueTickets,
        queues::MyQueuesResponse,
        queues::HandBackTicketRequest,
        queues::TakenBy,
        queues::QueueActivity,
        // search (3.9)
        search::SearchTicketsQuery,
        search::TextPart,
        search::TicketSearchHit,
        search::SearchHitKind,
        search::DocSearchHit,
        search::FacetCount,
        search::TicketSearchFacets,
        search::TicketSearchResponse,
        // ticket check (intel R1)
        ticket_check::TicketCheckFeature,
        ticket_check::SwitchLevel,
        ticket_check::QualityRule,
        ticket_check::HintSeverity,
        ticket_check::SecretKind,
        ticket_check::FixAction,
        ticket_check::CheckField,
        ticket_check::ReasonCode,
        ticket_check::Strength,
        ticket_check::ScoreBand,
        ticket_check::SuggestionKind,
        ticket_check::AbstainReason,
        ticket_check::Verdict,
        ticket_check::TicketCheckRequest,
        ticket_check::TicketCheckResponse,
        ticket_check::RecordDecisionsRequest,
        ticket_check::RecordDecisionsResponse,
        ticket_check::DeleteIntelDataResponse,
        ticket_check::FeatureDisabledDetails,
        // writing assistance, edit comparison, backlog review (assist)
        ticket_check::AssistLocale,
        ticket_check::AssistanceState,
        ticket_check::AssistanceScope,
        ticket_check::ModelCriterionState,
        ticket_check::ComparisonReason,
        ticket_check::AssistanceUnavailable,
        ticket_check::ReviewEngine,
        ticket_check::WritingAssistance,
        ticket_check::CompareEditsRequest,
        ticket_check::CompareEditsResponse,
        ticket_check::BacklogReviewQuery,
        ticket_check::BacklogReviewPage,
        // remembered review, trend, asking the reporter (review2)
        ticket_check::BacklogReviewRunRequest,
        ticket_check::BacklogReviewLastQuery,
        ticket_check::BacklogReviewLast,
        ticket_check::AskReporterRequest,
        ticket_check::AskReporterResponse,
        // triage suggestions (W3-E)
        ticket_check::TriageField,
        ticket_check::TriageReasonCode,
        ticket_check::TriageTicketRef,
        ticket_check::TriageReason,
        ticket_check::TriageSuggestion,
        // calendar: events and bands (UX9)
        calendar::EventWhen,
        calendar::EventResponse,
        calendar::CreateEventRequest,
        calendar::UpdateEventRequest,
        calendar::EventExpected,
        calendar::EventEdit,
        calendar::CalendarWindowQuery,
        calendar::CalendarNextSprint,
        calendar::CalendarSprintBand,
        calendar::CalendarRelease,
        calendar::CalendarResponse,
        edit::EditConflictDetails<calendar::EventResponse>,
        // Athene AI (AI1)
        athene_ai::AtheneAiAvailability,
        athene_ai::AtheneAiWorkspaceResponse,
        athene_ai::AtheneAiAdminResponse,
        athene_ai::UpdateAtheneAiRequest,
        athene_ai::EnrollAiWorkspaceRequest,
        // Athene AI Beta 0.2.0: the quality model's licences (Q020)
        athene_ai::AiModelLicensesResponse,
        // The quality model's versions and hot swap (models step)
        athene_ai::AiModelLicensesQuery,
        athene_ai::AiModelControl,
        athene_ai::AiModelVersion,
        athene_ai::AiModelCheck,
        athene_ai::AiModelSwap,
        athene_ai::AiWorkspaceModel,
    );
    // The verdicts the web's local rules are tested against (3.12).
    let matrix = policy::matrix::to_json(&policy::matrix::build()).expect("policy matrix");
    fs::create_dir_all(out.join("policy")).expect("policy dir");
    fs::write(out.join("policy/policy-matrix.json"), &matrix).expect("write policy matrix");
    // The same verdicts as a module the web's specs import (the package
    // ships `dist/` only): its loading fallback and its reading of the
    // capabilities are tested against them (3.12).
    let module = format!(
        "// Generated from athene_contracts::policy::matrix by tests/ts_bindings.rs. Do not edit this file manually.\n\
         import type {{ Action }} from \"./Action\";\n\
         import type {{ Permission }} from \"./Permission\";\n\
         import type {{ UserRole }} from \"../../payloads/auth\";\n\
         import type {{ WorkspaceMemberRole }} from \"../workspace/WorkspaceMemberRole\";\n\n\
         /** The facts one verdict of the matrix was given (absent = false / none). */\n\
         export interface PolicyMatrixFacts {{ unplaced?: boolean; owns?: boolean; target_is_self?: boolean; target_role?: WorkspaceMemberRole; granted_role?: WorkspaceMemberRole; granted_grants?: Permission[] }}\n\n\
         /** Every verdict of `allows()` for the matrix subjects (policy-matrix.json). */\n\
         export interface PolicyMatrix {{\n  revision: number;\n  link_invite_max_role: WorkspaceMemberRole;\n  actions: {{ [key in Action]?: {{ kind: 'read' | 'write'; scope: string; reads: string[]; permissions: Permission[] }} }};\n  subjects: {{ [name: string]: {{ platform_role: UserRole; membership: {{ tier: WorkspaceMemberRole; grants: Permission[] }} | null }} }};\n  cases: ReadonlyArray<{{ subject: string; action: Action; facts: PolicyMatrixFacts; allowed: boolean }}>;\n}}\n\n\
         // Parsed, not written as a literal: a literal of every case is one\n\
         // object type per case for the compiler to unite, past what it\n\
         // computes (TS2590 at 2,500 cases); the value is the same.\n\
         export const POLICY_MATRIX: PolicyMatrix = JSON.parse({});\n",
        serde_json::to_string(matrix.trim_end()).expect("matrix as a string literal")
    );
    fs::write(out.join("policy/policy-matrix.ts"), module).expect("write policy matrix module");
    // The override cases Rust and the web are both held to (phase B): the
    // concept's worked examples and the one-permission grid.
    let cases = overrides::cases::to_json().expect("override cases");
    fs::write(out.join("policy/override-cases.json"), &cases).expect("write override cases");
    let module = format!(
        "// Generated from athene_contracts::overrides::cases by tests/ts_bindings.rs. Do not edit this file manually.\n\
         import type {{ OverrideLevel }} from \"./OverrideLevel\";\n\
         import type {{ Permission }} from \"./Permission\";\n\
         import type {{ PermissionSource }} from \"./PermissionSource\";\n\
         import type {{ WorkspaceMemberRole }} from \"../workspace/WorkspaceMemberRole\";\n\n\
         /** One resolution case: who, through which levels, and what must come out. */\n\
         export interface OverrideCase {{ name: string; tier: WorkspaceMemberRole; base: Permission[]; levels: OverrideLevel[]; permission: Permission; allowed: boolean; source: PermissionSource; team_id?: string }}\n\n\
         /** The concept's worked examples and the one-permission grid (override-cases.json). */\n\
         export const OVERRIDE_CASES: {{ examples: ReadonlyArray<OverrideCase>; grid: ReadonlyArray<OverrideCase> }} = {};\n",
        cases.trim_end()
    );
    fs::write(out.join("policy/override-cases.ts"), module).expect("write override cases module");
    // The field-resolution cases Rust and the web are both held to (custom
    // fields CF1): the concept's precedence examples and the rule grid.
    let cases = field_assignments::cases::to_json().expect("field resolution cases");
    fs::create_dir_all(out.join("field_definitions")).expect("field definitions dir");
    fs::write(out.join("field_definitions/field-resolution.json"), &cases)
        .expect("write field resolution cases");
    let module = format!(
        "// Generated from athene_contracts::field_assignments::cases by tests/ts_bindings.rs. Do not edit this file manually.\n\
         import type {{ FieldContext }} from \"./FieldContext\";\n\
         import type {{ ResolutionDefinition }} from \"./ResolutionDefinition\";\n\
         import type {{ ResolutionRule }} from \"./ResolutionRule\";\n\
         import type {{ ResolvedField }} from \"./ResolvedField\";\n\n\
         /** One resolution case: a ticket, the workspace's fields and rules, and what must come out. */\n\
         export interface FieldResolutionCase {{ name: string; context: FieldContext; definitions: ResolutionDefinition[]; rules: ResolutionRule[]; values: Record<string, unknown>; expected: ResolvedField[] }}\n\n\
         /** The concept's precedence examples and the rule grid (field-resolution.json). */\n\
         export const FIELD_RESOLUTION_CASES: {{ examples: ReadonlyArray<FieldResolutionCase>; grid: ReadonlyArray<FieldResolutionCase> }} = {};\n",
        cases.trim_end()
    );
    fs::write(out.join("field_definitions/field-resolution.ts"), module)
        .expect("write field resolution module");
    // The permission catalogue (order, group, guest ceiling) and the role
    // templates, as values: the web's editor and its loading fallback read
    // them instead of a hand-kept copy (G2).
    fs::write(
        out.join("policy/permission-catalog.ts"),
        permission_catalog(),
    )
    .expect("write permission catalogue");
    // Invitation limits the web needs at run time, from the Rust constants
    // rather than a hand-kept copy (G2).
    let constants = format!(
        "// Generated from the Rust constants by tests/ts_bindings.rs. Do not edit this file manually.\n\
         import type {{ WorkspaceMemberRole }} from \"./WorkspaceMemberRole\";\n\n\
         /** The most an open link may grant (J6); admins and owners are invited by address. */\n\
         export const LINK_INVITE_MAX_ROLE: WorkspaceMemberRole = {};\n\n\
         /** Days an invitation lives when the request names no expiry (J4). */\n\
         export const INVITE_DEFAULT_DAYS = {};\n\n\
         /** The longest an invitation may live, in days (J4). */\n\
         export const INVITE_MAX_DAYS = {};\n",
        serde_json::to_string(&policy::LINK_INVITE_MAX_ROLE).expect("role"),
        workspace::INVITE_DEFAULT_DAYS,
        workspace::INVITE_MAX_DAYS,
    );
    fs::write(out.join("workspace/invite-constants.ts"), constants).expect("write constants");
    // The event limits the web checks before it sends (UX9), as values.
    let calendar = format!(
        "// Generated from the Rust constants by tests/ts_bindings.rs. Do not edit this file manually.\n\n\
         /** The longest event title, in characters. */\n\
         export const EVENT_MAX_TITLE_CHARS = {};\n\n\
         /** The longest event description, in characters. */\n\
         export const EVENT_MAX_DESCRIPTION_CHARS = {};\n\n\
         /** At most this many participants per event. */\n\
         export const EVENT_MAX_PARTICIPANTS = {};\n\n\
         /** The longest an event may run, in days. */\n\
         export const EVENT_MAX_SPAN_DAYS = {};\n\n\
         /** The widest window a calendar read may ask for, in days (`to - from`). */\n\
         export const CALENDAR_MAX_WINDOW_DAYS = {};\n",
        calendar::EVENT_MAX_TITLE_CHARS,
        calendar::EVENT_MAX_DESCRIPTION_CHARS,
        calendar::EVENT_MAX_PARTICIPANTS,
        calendar::EVENT_MAX_SPAN_DAYS,
        calendar::CALENDAR_MAX_WINDOW_DAYS,
    );
    fs::create_dir_all(out.join("calendar")).expect("calendar dir");
    fs::write(out.join("calendar/calendar-constants.ts"), calendar).expect("write constants");
    // Every activity verb as a value, so the web can check a wire string
    // against the closed set (BE-17, 3.10).
    let verbs: Vec<String> = activity::ActivityAction::ALL
        .iter()
        .map(|action| format!("  \"{}\",\n", action.as_str()))
        .collect();
    let actions = format!(
        "// Generated from the Rust constants by tests/ts_bindings.rs. Do not edit this file manually.\n\
         import type {{ ActivityAction }} from \"./ActivityAction\";\n\n\
         /** Every verb the server records (`ActivityAction::ALL`). */\n\
         export const ACTIVITY_ACTIONS: readonly ActivityAction[] = [\n{}];\n",
        verbs.concat()
    );
    fs::write(out.join("activity/activity-actions.ts"), actions).expect("write actions");
    fs::create_dir_all(out.join("limits")).expect("limits dir");
    fs::write(out.join("limits/limits.ts"), limits_ts()).expect("write limits");
    fs::write(
        out.join("attachments/attachment-types.ts"),
        attachment_types_ts(),
    )
    .expect("write attachment types");
}

/// `attachments/attachment-types.ts`: the allowlist of
/// `athene_contracts::attachments`, for a file picker's `accept` and the
/// icons.
fn attachment_types_ts() -> String {
    use athene_contracts::attachments::{ATTACHMENT_NAME_HEADER, ATTACHMENT_TYPES};
    let mut out = format!(
        "// Generated from the Rust constants by tests/ts_bindings.rs. Do not edit this file manually.\n\
         import type {{ AttachmentKind }} from \"./AttachmentKind\";\n\n\
         /** The request header of an upload that carries the file name (UTF-8, percent-encoded). */\n\
         export const ATTACHMENT_NAME_HEADER = '{ATTACHMENT_NAME_HEADER}';\n\n\
         /** The types the server accepts (sniffed from the bytes), with their extensions. */\n\
         export const ATTACHMENT_TYPES: ReadonlyArray<{{ contentType: string; kind: AttachmentKind; extensions: readonly string[]; inline: boolean }}> = [\n"
    );
    for t in ATTACHMENT_TYPES {
        let kind = serde_json::to_string(&t.kind)
            .expect("kind")
            .replace('"', "'");
        let extensions = t
            .extensions
            .iter()
            .map(|e| format!("'{e}'"))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "  {{ contentType: '{}', kind: {kind}, extensions: [{extensions}], inline: {} }},\n",
            t.content_type, t.inline
        ));
    }
    out.push_str("];\n");
    out
}

/// `limits/limits.ts`: the length limits of `athene_contracts::limits`.
fn limits_ts() -> String {
    use athene_contracts::limits::*;
    let rows: &[(&str, &str, usize)] = &[
        (
            "TICKET_TITLE_MIN_CHARS",
            "A ticket title, after trimming.",
            TICKET_TITLE_MIN_CHARS,
        ),
        (
            "TICKET_TITLE_MAX_CHARS",
            "A ticket title, after trimming.",
            TICKET_TITLE_MAX_CHARS,
        ),
        (
            "TICKET_DESCRIPTION_MAX_BYTES",
            "A ticket description, in UTF-8 bytes (it must fit through the host's connector).",
            TICKET_DESCRIPTION_MAX_BYTES,
        ),
        (
            "COMMENT_MIN_CHARS",
            "A comment, after trimming.",
            COMMENT_MIN_CHARS,
        ),
        (
            "COMMENT_MAX_CHARS",
            "A comment, after trimming.",
            COMMENT_MAX_CHARS,
        ),
        (
            "WORK_LOG_NOTE_MAX_CHARS",
            "The note of a time booking.",
            WORK_LOG_NOTE_MAX_CHARS,
        ),
        (
            "WORKSPACE_NAME_MAX_CHARS",
            "A workspace's name.",
            WORKSPACE_NAME_MAX_CHARS,
        ),
        (
            "SPRINT_NAME_MAX_CHARS",
            "A sprint's name.",
            SPRINT_NAME_MAX_CHARS,
        ),
        (
            "RELEASE_NAME_MAX_CHARS",
            "A release's name.",
            RELEASE_NAME_MAX_CHARS,
        ),
        (
            "LABEL_NAME_MAX_CHARS",
            "A label's name.",
            LABEL_NAME_MAX_CHARS,
        ),
        (
            "TICKET_LIST_QUERY_MAX_CHARS",
            "The search of a paged ticket list (`q`).",
            athene_contracts::listing::TICKET_LIST_QUERY_MAX_CHARS,
        ),
        (
            "TICKET_PAGE_DEFAULT_LIMIT",
            "Tickets per page when a paged list names no `limit`.",
            athene_contracts::listing::TICKET_PAGE_DEFAULT_LIMIT as usize,
        ),
        (
            "TICKET_PAGE_MAX_LIMIT",
            "The most tickets one page holds.",
            athene_contracts::listing::TICKET_PAGE_MAX_LIMIT as usize,
        ),
        (
            "TICKET_LIST_MAX_FIELD_FILTERS",
            "The most custom-field conditions one list takes (`cf`).",
            athene_contracts::listing::TICKET_LIST_MAX_FIELD_FILTERS,
        ),
        (
            "TICKET_LIST_FIELD_FILTERS_MAX_CHARS",
            "The longest `cf` parameter of a list (its JSON).",
            athene_contracts::listing::TICKET_LIST_FIELD_FILTERS_MAX_CHARS,
        ),
        (
            "TICKET_LIST_MAX_FIELD_KEYS",
            "The most custom fields a list summary carries values of (`cf_keys`).",
            athene_contracts::listing::TICKET_LIST_MAX_FIELD_KEYS,
        ),
        (
            "ATTACHMENT_MAX_BYTES",
            "One attachment, in bytes.",
            ATTACHMENT_MAX_BYTES,
        ),
        (
            "ATTACHMENT_WORKSPACE_MAX_BYTES",
            "All attachments of one workspace together, in bytes (trash included).",
            ATTACHMENT_WORKSPACE_MAX_BYTES,
        ),
        (
            "ATTACHMENTS_PER_TICKET_MAX",
            "Attachments of one ticket and its comments, not counting the trash.",
            ATTACHMENTS_PER_TICKET_MAX,
        ),
        (
            "ATTACHMENT_NAME_MAX_CHARS",
            "An attachment's name, after sanitising.",
            ATTACHMENT_NAME_MAX_CHARS,
        ),
    ];
    let mut out = String::from(
        "// Generated from the Rust constants by tests/ts_bindings.rs. Do not edit this file manually.\n\
         // Lengths count characters (code points) unless the name says bytes.\n",
    );
    for (name, doc, value) in rows {
        out.push_str(&format!("\n/** {doc} */\nexport const {name} = {value};\n"));
    }
    out
}

/// `policy/permission-catalog.ts`.
fn permission_catalog() -> String {
    use athene_contracts::policy::{
        Permission, PermissionSet, RolePreset, GUEST_CEILING, MEMBER_DEFAULT, PLATFORM_MEMBER_EXTRA,
    };
    let names = |set: PermissionSet| {
        set.iter()
            .map(|p| format!("'{}'", p.as_str()))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut out = String::from(
        "// Generated from the Rust constants by tests/ts_bindings.rs. Do not edit this file manually.\n\
         import type { Permission } from \"./Permission\";\n\
         import type { PermissionGroup } from \"./PermissionGroup\";\n\
         import type { RolePreset } from \"./RolePreset\";\n\n\
         /** Every permission in the role editor's order, with its section. */\n\
         export const PERMISSIONS: ReadonlyArray<{ permission: Permission; group: PermissionGroup; guest: boolean; effective: boolean }> = [\n",
    );
    for p in Permission::ALL {
        out.push_str(&format!(
            "  {{ permission: '{}', group: {}, guest: {}, effective: {} }},\n",
            p.as_str(),
            serde_json::to_string(&p.group())
                .expect("group")
                .replace('"', "'"),
            p.guest_allowed(),
            p.effective_yet(),
        ));
    }
    out.push_str("];\n\n");
    out.push_str(&format!(
        "/** What a guest role can ever hold (`GUEST_CEILING`). */\n\
         export const GUEST_CEILING: ReadonlyArray<Permission> = [{}];\n\n\
         /** The role Mitglied of a new workspace (`MEMBER_DEFAULT`). */\n\
         export const MEMBER_DEFAULT: ReadonlyArray<Permission> = [{}];\n\n\
         /** What a platform operator or admin who is a member keeps (`PLATFORM_MEMBER_EXTRA`, E7). */\n\
         export const PLATFORM_MEMBER_EXTRA: ReadonlyArray<Permission> = [{}];\n\n",
        names(GUEST_CEILING),
        names(MEMBER_DEFAULT),
        names(PLATFORM_MEMBER_EXTRA),
    ));
    out.push_str(&format!(
        "/** What an override may name at each place (phase B, CONCEPT §3). */\n\
         export const OVERRIDABLE: Readonly<Record<'workspace' | 'project' | 'doc_collection', ReadonlyArray<Permission>>> = {{\n  workspace: [{}],\n  project: [{}],\n  doc_collection: [{}],\n}};\n\n",
        names(athene_contracts::overrides::WORKSPACE_OVERRIDABLE),
        names(athene_contracts::overrides::PROJECT_OVERRIDABLE),
        names(athene_contracts::overrides::DOC_COLLECTION_OVERRIDABLE),
    ));
    out.push_str("/** The role templates' ticks. */\nexport const ROLE_PRESETS: Readonly<Record<RolePreset, ReadonlyArray<Permission>>> = {\n");
    for preset in RolePreset::ALL {
        out.push_str(&format!(
            "  {}: [{}],\n",
            preset.as_str(),
            names(preset.permissions())
        ));
    }
    out.push_str("};\n\n");
    out.push_str(&format!(
        "/** The rules' revision (`POLICY_REVISION`): what this package's matrix was made with. */\nexport const POLICY_REVISION = {};\n\n",
        athene_contracts::policy::POLICY_REVISION
    ));
    out.push_str(&format!(
        "/** The longest role name, in characters. */\nexport const ROLE_MAX_NAME_CHARS = {};\n\n\
         /** The longest role description, in characters. */\nexport const ROLE_MAX_DESCRIPTION_CHARS = {};\n\n\
         /** The most roles a workspace may hold, system roles included. */\nexport const ROLE_MAX_PER_WORKSPACE = {};\n",
        athene_contracts::roles::ROLE_MAX_NAME_CHARS,
        athene_contracts::roles::ROLE_MAX_DESCRIPTION_CHARS,
        athene_contracts::roles::ROLE_MAX_PER_WORKSPACE,
    ));
    out
}

/// Every file under `root`, keyed by its `/`-separated path relative to it.
fn files(root: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .expect("under root")
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                out.insert(rel, fs::read_to_string(&path).expect("utf-8 file"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

#[test]
fn generated_typescript_is_current() {
    let fresh_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("athene-contracts-ts");
    let _ = fs::remove_dir_all(&fresh_dir);
    render(&fresh_dir);
    let fresh = files(&fresh_dir);
    assert!(!fresh.is_empty(), "no bindings were generated");

    let committed_root = committed_dir();
    if std::env::var_os(REGENERATE_ENV).is_some() {
        let _ = fs::remove_dir_all(&committed_root);
        for (rel, contents) in &fresh {
            let path = committed_root.join(rel);
            fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
            fs::write(&path, contents).expect("write binding");
        }
        return;
    }

    let committed = files(&committed_root);
    let mut problems = Vec::new();
    for (rel, contents) in &fresh {
        match committed.get(rel) {
            None => problems.push(format!("missing: {rel}")),
            Some(existing) if existing != contents => problems.push(format!("differs: {rel}")),
            Some(_) => {}
        }
    }
    for rel in committed.keys() {
        if !fresh.contains_key(rel) {
            problems.push(format!("stale (no Rust type generates it): {rel}"));
        }
    }
    assert!(
        problems.is_empty(),
        "src/generated is out of date with the Rust contracts:\n  {}\n\
         Regenerate with `{REGENERATE_ENV}=1 cargo test -p athene-contracts --test ts_bindings` \
         (or `npm run generate`) and commit the result.",
        problems.join("\n  ")
    );
}
