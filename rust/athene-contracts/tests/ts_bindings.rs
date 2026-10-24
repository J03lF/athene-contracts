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
//! so a rule change shows up as a diff of that file too.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use athene_contracts::{
    activity, admin, auth, common, dashboard, docs, edit, field_definitions, policy, teams,
    ticket_types, user, work, workspace,
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
        activity::ActivityEventResponse,
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
        policy::CapabilitiesResponse,
    );
    // The verdicts the web's local rules are tested against (3.12).
    let matrix = policy::matrix::to_json(&policy::matrix::build()).expect("policy matrix");
    fs::create_dir_all(out.join("policy")).expect("policy dir");
    fs::write(out.join("policy/policy-matrix.json"), matrix).expect("write policy matrix");
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
