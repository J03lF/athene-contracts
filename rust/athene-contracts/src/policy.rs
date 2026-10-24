//! The permission policy (roadmap step 3.8a, `POLICY_PLAN.md` §2): what a
//! person may do in Athene, as one table of actions over one standing rule.
//!
//! Pure functions, no IO. The core loads the facts — the platform role from
//! the signed principal, the membership from one row, ownership from the
//! resource — and asks [`allows`]; `GET /work/workspaces/:id/capabilities`
//! answers the same question for every action at once ([`capabilities`]);
//! the web checks its local rules against the generated matrix
//! (`src/generated/policy/policy-matrix.json`, see [`matrix`]).
//!
//! Until 3.8a the rule lived in five copies (`WorkAccessService::standing`,
//! the teams directory, ticket types, docs, `WorkspaceService::require_
//! admin`) and the web's `policy.ts`, and they disagreed in places. Now:
//!
//! - **Standing** ([`standing`]): a workspace guest reads; an owner or admin
//!   manages; a member contributes, and manages when they are a platform
//!   operator or admin. Data that belongs to no workspace is the platform
//!   admin's alone.
//! - **Governing** ([`governs`], J1): invitations, roles, removing people,
//!   the workspace's name and settings are for its *own* owners and admins.
//!   A platform operator or admin who is a member manages content (teams,
//!   labels, types, fields) but does not govern.
//! - **Platform guests write nothing** (D3), wherever they are a member.
//!
//! A new action is a variant of [`Action`] plus its arm in the rule table:
//! the match is exhaustive, so an action without a rule does not compile.
//! Changing a rule changes the matrix, and `tests/policy_matrix.rs` then
//! asks for [`POLICY_REVISION`] + 1, which the capabilities answer carries
//! so a client can tell its cached rules are stale.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::auth::UserRole;
use crate::workspace::WorkspaceMemberRole;

/// The rules' revision. Bumped whenever a verdict in the matrix changes
/// (`tests/policy_matrix.rs` keeps the history).
pub const POLICY_REVISION: u32 = 3;

/// The most an open link may grant (J6). Admins and owners are invited by
/// email only: a forwarded link must not hand out the workspace.
pub const LINK_INVITE_MAX_ROLE: WorkspaceMemberRole = WorkspaceMemberRole::Member;

/// Every action the server authorises and the web asks about. The wire
/// names are the web's (`ticket.edit`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS,
)]
#[ts(export_to = "policy/")]
pub enum Action {
    /// The platform admin console.
    #[serde(rename = "admin.access")]
    AdminAccess,
    #[serde(rename = "workspace.create")]
    WorkspaceCreate,
    /// The instance's SLA policies (`/work/slas`).
    #[serde(rename = "sla.read")]
    SlaRead,
    #[serde(rename = "sla.manage")]
    SlaManage,
    /// Every read inside a workspace: the workspace, its projects,
    /// tickets, sprints, releases, reports, docs, labels.
    #[serde(rename = "workspace.read")]
    WorkspaceRead,
    /// Name, description, colour and settings.
    #[serde(rename = "workspace.settings.edit")]
    WorkspaceSettingsEdit,
    /// Facts: `granted_role` (none = any the actor may grant).
    #[serde(rename = "invite.create")]
    InviteCreate,
    /// List, revoke and renew invitations.
    #[serde(rename = "invite.manage")]
    InviteManage,
    /// Facts: `target_role`, `granted_role`, `target_is_self`.
    #[serde(rename = "members.role.change")]
    MembersRoleChange,
    /// Facts: `target_role`, `target_is_self` (leaving).
    #[serde(rename = "members.remove")]
    MembersRemove,
    #[serde(rename = "teams.manage")]
    TeamsManage,
    /// Facts: `target_is_self` (leaving a team).
    #[serde(rename = "teams.member.remove")]
    TeamsMemberRemove,
    #[serde(rename = "labels.manage")]
    LabelsManage,
    #[serde(rename = "ticket_types.manage")]
    TicketTypesManage,
    #[serde(rename = "fields.manage")]
    FieldsManage,
    #[serde(rename = "doc.write")]
    DocWrite,
    /// Facts: `owns` (the author).
    #[serde(rename = "doc.delete")]
    DocDelete,
    #[serde(rename = "project.create")]
    ProjectCreate,
    /// Facts: `owns` (creator or lead).
    #[serde(rename = "project.edit")]
    ProjectEdit,
    /// Sprints and releases themselves: create, edit, start, complete,
    /// delete. Managers only, whatever the workspace's `ticket_planning`.
    #[serde(rename = "project.plan")]
    ProjectPlan,
    /// Put a ticket into a sprint or a release, or take it out. Facts:
    /// `editors_plan` (the workspace's `ticket_planning` is `editors`).
    #[serde(rename = "ticket.plan")]
    TicketPlan,
    #[serde(rename = "ticket.create")]
    TicketCreate,
    /// Edit, move, assign, triage, label, link.
    #[serde(rename = "ticket.edit")]
    TicketEdit,
    /// Facts: `owns` (the reporter).
    #[serde(rename = "ticket.delete")]
    TicketDelete,
    #[serde(rename = "ticket.watch")]
    TicketWatch,
    #[serde(rename = "comment.create")]
    CommentCreate,
    /// Facts: `owns` (the author).
    #[serde(rename = "comment.edit")]
    CommentEdit,
    /// Facts: `owns` (the author).
    #[serde(rename = "comment.delete")]
    CommentDelete,
    /// Delete other people's comments (a context-free hint for the UI).
    #[serde(rename = "comment.moderate")]
    CommentModerate,
    #[serde(rename = "time.log")]
    TimeLog,
    /// Facts: `owns` (the author).
    #[serde(rename = "time.delete")]
    TimeDelete,
    /// Edit or delete other people's time entries.
    #[serde(rename = "time.moderate")]
    TimeModerate,
    /// Have a draft checked and record what one did with the suggestions
    /// (ticket check, revision 2).
    #[serde(rename = "ticket.check")]
    TicketCheck,
}

/// Whether an action changes something. Platform guests do no `Write`
/// anywhere, which is what the route layer's `require_user_write` says too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "policy/")]
pub enum ActionKind {
    Read,
    Write,
}

/// What an action is checked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "policy/")]
pub enum ActionScope {
    /// The platform role alone; no workspace involved.
    Platform,
    Workspace,
    Project,
    Ticket,
}

/// A fact about the resource (or the person acted on) an action's rule
/// reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "policy/")]
pub enum Fact {
    /// The actor authored, reported, created or leads the resource.
    Owns,
    /// The person acted on is the actor.
    TargetIsSelf,
    /// The role the person acted on holds now.
    TargetRole,
    /// The role an invitation or a role change would hand out.
    GrantedRole,
    /// The workspace lets everyone who edits a ticket plan it
    /// (`WorkspaceSettings::ticket_planning` = `editors`, the default).
    EditorsPlan,
}

impl Action {
    /// Every action, in wire order.
    pub const ALL: [Action; 33] = [
        Action::AdminAccess,
        Action::WorkspaceCreate,
        Action::SlaRead,
        Action::SlaManage,
        Action::WorkspaceRead,
        Action::WorkspaceSettingsEdit,
        Action::InviteCreate,
        Action::InviteManage,
        Action::MembersRoleChange,
        Action::MembersRemove,
        Action::TeamsManage,
        Action::TeamsMemberRemove,
        Action::LabelsManage,
        Action::TicketTypesManage,
        Action::FieldsManage,
        Action::DocWrite,
        Action::DocDelete,
        Action::ProjectCreate,
        Action::ProjectEdit,
        Action::ProjectPlan,
        Action::TicketCreate,
        Action::TicketEdit,
        Action::TicketDelete,
        Action::TicketWatch,
        Action::CommentCreate,
        Action::CommentEdit,
        Action::CommentDelete,
        Action::CommentModerate,
        Action::TimeLog,
        Action::TimeDelete,
        Action::TimeModerate,
        Action::TicketCheck,
        Action::TicketPlan,
    ];

    /// The wire name (`ticket.edit`), the same string serde writes.
    pub const fn as_str(self) -> &'static str {
        match self {
            Action::AdminAccess => "admin.access",
            Action::WorkspaceCreate => "workspace.create",
            Action::SlaRead => "sla.read",
            Action::SlaManage => "sla.manage",
            Action::WorkspaceRead => "workspace.read",
            Action::WorkspaceSettingsEdit => "workspace.settings.edit",
            Action::InviteCreate => "invite.create",
            Action::InviteManage => "invite.manage",
            Action::MembersRoleChange => "members.role.change",
            Action::MembersRemove => "members.remove",
            Action::TeamsManage => "teams.manage",
            Action::TeamsMemberRemove => "teams.member.remove",
            Action::LabelsManage => "labels.manage",
            Action::TicketTypesManage => "ticket_types.manage",
            Action::FieldsManage => "fields.manage",
            Action::DocWrite => "doc.write",
            Action::DocDelete => "doc.delete",
            Action::ProjectCreate => "project.create",
            Action::ProjectEdit => "project.edit",
            Action::ProjectPlan => "project.plan",
            Action::TicketCreate => "ticket.create",
            Action::TicketEdit => "ticket.edit",
            Action::TicketDelete => "ticket.delete",
            Action::TicketWatch => "ticket.watch",
            Action::CommentCreate => "comment.create",
            Action::CommentEdit => "comment.edit",
            Action::CommentDelete => "comment.delete",
            Action::CommentModerate => "comment.moderate",
            Action::TimeLog => "time.log",
            Action::TimeDelete => "time.delete",
            Action::TimeModerate => "time.moderate",
            Action::TicketCheck => "ticket.check",
            Action::TicketPlan => "ticket.plan",
        }
    }

    pub const fn kind(self) -> ActionKind {
        match self {
            Action::AdminAccess | Action::SlaRead | Action::WorkspaceRead => ActionKind::Read,
            _ => ActionKind::Write,
        }
    }

    pub const fn scope(self) -> ActionScope {
        match self {
            Action::AdminAccess | Action::WorkspaceCreate | Action::SlaRead | Action::SlaManage => {
                ActionScope::Platform
            }
            Action::ProjectEdit | Action::ProjectPlan | Action::TicketCreate => {
                ActionScope::Project
            }
            Action::TicketEdit
            | Action::TicketPlan
            | Action::TicketDelete
            | Action::TicketWatch
            | Action::CommentCreate
            | Action::CommentEdit
            | Action::CommentDelete
            | Action::CommentModerate
            | Action::TimeLog
            | Action::TimeDelete
            | Action::TimeModerate => ActionScope::Ticket,
            _ => ActionScope::Workspace,
        }
    }

    /// The facts the action's rule reads; empty for a context-free action.
    pub const fn reads(self) -> &'static [Fact] {
        match self {
            Action::InviteCreate => &[Fact::GrantedRole],
            Action::MembersRoleChange => &[Fact::TargetRole, Fact::GrantedRole, Fact::TargetIsSelf],
            Action::MembersRemove => &[Fact::TargetRole, Fact::TargetIsSelf],
            Action::TeamsMemberRemove => &[Fact::TargetIsSelf],
            Action::DocDelete
            | Action::ProjectEdit
            | Action::TicketDelete
            | Action::CommentEdit
            | Action::CommentDelete
            | Action::TimeDelete => &[Fact::Owns],
            Action::TicketPlan => &[Fact::EditorsPlan],
            _ => &[],
        }
    }

    /// Whether the verdict depends on the resource or the person acted on
    /// (see [`Action::reads`]). A workspace fact (`editors_plan`) is not
    /// context: the capabilities answer it per workspace.
    pub const fn contextual(self) -> bool {
        !matches!(self.reads(), [] | [Fact::EditorsPlan])
    }
}

/// What someone may do in one workspace, weakest first.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "policy/")]
pub enum Standing {
    /// Reads and comments (a workspace guest).
    Reader,
    /// Files and changes tickets (a member).
    Contributor,
    /// Plans, moderates and sets the workspace's content up (an owner or
    /// admin, or a platform operator or admin who is a member).
    Manager,
}

/// Who is asking: the platform role (from the signed principal) and the
/// role in the workspace in question, if any.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subject {
    pub platform_role: UserRole,
    /// `None`: not a member (or the resource belongs to no workspace).
    pub membership: Option<WorkspaceMemberRole>,
}

impl Subject {
    pub fn new(platform_role: UserRole, membership: Option<WorkspaceMemberRole>) -> Self {
        Self {
            platform_role,
            membership,
        }
    }
}

/// What a contextual rule needs to know about the resource. The default is
/// "somebody else's, in a workspace, no roles named".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facts {
    /// The resource belongs to no workspace (a project from before
    /// workspaces, migration 0011).
    #[serde(default, skip_serializing_if = "is_false")]
    pub unplaced: bool,
    /// The actor authored, reported, created or leads it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub owns: bool,
    /// The person acted on is the actor.
    #[serde(default, skip_serializing_if = "is_false")]
    pub target_is_self: bool,
    /// The role the person acted on holds; `None` with `target_is_self`
    /// means the actor's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_role: Option<WorkspaceMemberRole>,
    /// The role being handed out; `None` = any the actor may grant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granted_role: Option<WorkspaceMemberRole>,
    /// The workspace lets editors plan tickets (`ticket_planning` =
    /// `editors`). A workspace fact, the same for every resource in it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub editors_plan: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl Facts {
    /// Nothing known: somebody else's resource in a workspace.
    pub const NONE: Facts = Facts {
        unplaced: false,
        owns: false,
        target_is_self: false,
        target_role: None,
        granted_role: None,
        editors_plan: false,
    };

    pub const fn owned(owns: bool) -> Self {
        Facts { owns, ..Self::NONE }
    }

    pub const fn unplaced() -> Self {
        Facts {
            unplaced: true,
            ..Self::NONE
        }
    }

    /// Acting on a person: their role now and whether it is the actor.
    pub const fn target(role: Option<WorkspaceMemberRole>, is_self: bool) -> Self {
        Facts {
            target_role: role,
            target_is_self: is_self,
            ..Self::NONE
        }
    }

    pub const fn granting(mut self, role: WorkspaceMemberRole) -> Self {
        self.granted_role = Some(role);
        self
    }

    /// The workspace's `ticket_planning` (true: `editors`).
    pub const fn planning(mut self, editors_plan: bool) -> Self {
        self.editors_plan = editors_plan;
        self
    }
}

/// The standing rule — the only one.
///
/// - `unplaced`: nobody can be a member of "no workspace", so only the
///   platform admin, the one authority that is not workspace-scoped, gets
///   `Manager`; everyone else `None` (operators too: adopting such a
///   project is an admin action).
/// - not a member: `None` — the caller answers `403`, exactly as for an id
///   that does not exist.
pub fn standing(subject: &Subject, unplaced: bool) -> Option<Standing> {
    if unplaced {
        return matches!(subject.platform_role, UserRole::Admin).then_some(Standing::Manager);
    }
    Some(match subject.membership? {
        WorkspaceMemberRole::Guest => Standing::Reader,
        WorkspaceMemberRole::Owner | WorkspaceMemberRole::Admin => Standing::Manager,
        WorkspaceMemberRole::Member
            if matches!(subject.platform_role, UserRole::Operator | UserRole::Admin) =>
        {
            Standing::Manager
        }
        WorkspaceMemberRole::Member => Standing::Contributor,
    })
}

/// Whether the subject runs the workspace: its own owners and admins, not
/// platform operators or admins who are members (J1).
pub fn governs(subject: &Subject) -> bool {
    matches!(
        subject.membership,
        Some(WorkspaceMemberRole::Owner | WorkspaceMemberRole::Admin)
    )
}

/// The verdict.
pub fn allows(subject: &Subject, action: Action, facts: &Facts) -> bool {
    // A platform guest changes nothing, wherever they are a member (D3).
    if action.kind() == ActionKind::Write && subject.platform_role == UserRole::Guest {
        return false;
    }
    match action {
        Action::AdminAccess => matches!(subject.platform_role, UserRole::Admin),
        Action::WorkspaceCreate => true,
        Action::SlaRead | Action::SlaManage => {
            matches!(subject.platform_role, UserRole::Operator | UserRole::Admin)
        }
        _ => match standing(subject, facts.unplaced) {
            Some(standing) => in_workspace(subject, standing, action, facts),
            None => false,
        },
    }
}

/// The rule table for everything inside a workspace.
fn in_workspace(subject: &Subject, standing: Standing, action: Action, facts: &Facts) -> bool {
    let manager = standing == Standing::Manager;
    let contributor = standing >= Standing::Contributor;
    match action {
        // Decided on the platform role alone, in `allows`.
        Action::AdminAccess | Action::WorkspaceCreate | Action::SlaRead | Action::SlaManage => {
            false
        }
        // Guests read, watch and comment.
        Action::WorkspaceRead | Action::TicketWatch | Action::CommentCreate => true,
        Action::WorkspaceSettingsEdit | Action::InviteManage => governs(subject),
        // Nobody hands out more than they hold (an owner invite only by an
        // owner).
        Action::InviteCreate => governs(subject) && within_own(subject, facts.granted_role),
        Action::MembersRoleChange => changes_role(subject, facts),
        // Leaving is everyone's own call; removing others is governing.
        Action::MembersRemove => {
            facts.target_is_self || (governs(subject) && may_touch(subject, facts.target_role))
        }
        Action::TeamsManage
        | Action::LabelsManage
        | Action::TicketTypesManage
        | Action::FieldsManage
        | Action::ProjectPlan
        | Action::CommentModerate
        | Action::TimeModerate => manager,
        Action::TeamsMemberRemove => manager || facts.target_is_self,
        Action::DocWrite
        | Action::ProjectCreate
        | Action::TicketCreate
        | Action::TicketEdit
        | Action::TicketCheck
        | Action::TimeLog => contributor,
        // Into a sprint or a release: managers, and editors where the
        // workspace says so (`ticket_planning`, default `editors`).
        Action::TicketPlan => manager || (contributor && facts.editors_plan),
        // Deleting has no undo: managers anything, contributors their own.
        Action::DocDelete | Action::ProjectEdit | Action::TicketDelete | Action::TimeDelete => {
            manager || (contributor && facts.owns)
        }
        // Your words are yours, at every standing.
        Action::CommentEdit => facts.owns,
        Action::CommentDelete => facts.owns || manager,
    }
}

/// `members.role.change`: governing; an owner's role, and the owner role,
/// only by an owner; never above the actor's own role; one's own role only
/// downwards (J7). The last owner is the repository's to protect (409).
fn changes_role(subject: &Subject, facts: &Facts) -> bool {
    if !governs(subject)
        || !may_touch(subject, facts.target_role)
        || !within_own(subject, facts.granted_role)
    {
        return false;
    }
    if facts.target_is_self {
        let current = facts.target_role.or(subject.membership);
        return match (facts.granted_role, current) {
            (Some(granted), Some(current)) => granted.rank() <= current.rank(),
            (Some(_), None) => false,
            (None, _) => true,
        };
    }
    true
}

/// An owner is touched only by an owner.
fn may_touch(subject: &Subject, target: Option<WorkspaceMemberRole>) -> bool {
    target != Some(WorkspaceMemberRole::Owner)
        || subject.membership == Some(WorkspaceMemberRole::Owner)
}

/// The granted role is no more than the subject's own (none named: fine).
fn within_own(subject: &Subject, granted: Option<WorkspaceMemberRole>) -> bool {
    match (granted, subject.membership) {
        (None, _) => true,
        (Some(granted), Some(own)) => granted.rank() <= own.rank(),
        (Some(_), None) => false,
    }
}

/// The roles the subject may hand out by invitation or role change, lowest
/// first; empty for whoever does not govern.
pub fn grantable_roles(subject: &Subject) -> Vec<WorkspaceMemberRole> {
    let Some(own) = subject.membership.filter(|_| governs(subject)) else {
        return Vec::new();
    };
    let mut roles: Vec<WorkspaceMemberRole> = WorkspaceMemberRole::ALL
        .into_iter()
        .filter(|role| role.rank() <= own.rank())
        .collect();
    roles.sort_by_key(|role| role.rank());
    roles
}

/// Whether an open link may grant `role` (J6): at most
/// [`LINK_INVITE_MAX_ROLE`].
pub fn link_invite_allows(role: WorkspaceMemberRole) -> bool {
    role.rank() <= LINK_INVITE_MAX_ROLE.rank()
}

/// Who work may be assigned to (J8, D7): any member of its workspace,
/// guests included — they read and comment, and may well be the person the
/// work waits on — but never a platform guest, who changes nothing. Work
/// outside any workspace has no roster: anyone but a platform guest.
pub fn assignable(assignee: &Subject, unplaced: bool) -> bool {
    assignee.platform_role != UserRole::Guest && (unplaced || assignee.membership.is_some())
}

/// `GET /work/workspaces/:workspace_id/capabilities`: the caller's verdict
/// on every action in one workspace, in `{ "data": … }`.
///
/// Answered with `Cache-Control: no-store`; a membership change counts from
/// the next request, a platform role change within the identity cache's
/// 30 s.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "policy/")]
pub struct CapabilitiesResponse {
    pub workspace_id: String,
    pub standing: Standing,
    pub role: WorkspaceMemberRole,
    /// Every action. A contextual one is `true` when it is allowed on any
    /// resource (somebody else's, no role named); the platform actions are
    /// included.
    #[schema(value_type = BTreeMap<String, bool>)]
    pub actions: BTreeMap<Action, bool>,
    /// Contextual actions only: allowed on the caller's own resources (or
    /// on themselves). `can(a, ctx) = actions[a] || (on_own[a] && isOwn(ctx))`.
    #[schema(value_type = BTreeMap<String, bool>)]
    pub on_own: BTreeMap<Action, bool>,
    /// For `invite.create` and `members.role.change`, lowest first.
    pub grantable_roles: Vec<WorkspaceMemberRole>,
    /// The most an open link may grant (J6).
    pub link_invite_max_role: WorkspaceMemberRole,
    /// The rules' revision (`POLICY_REVISION`); a client holding rules of
    /// another revision reloads them.
    pub policy_revision: u32,
}

/// The capabilities of a member; `None` for a non-member (the endpoint
/// answers `403`, as for a workspace that does not exist). `editors_plan`
/// is the workspace's `ticket_planning` (true: `editors`), so `ticket.plan`
/// is answered for this workspace.
pub fn capabilities(
    subject: &Subject,
    workspace_id: String,
    editors_plan: bool,
) -> Option<CapabilitiesResponse> {
    let role = subject.membership?;
    let standing = standing(subject, false)?;
    let workspace = Facts::NONE.planning(editors_plan);
    let actions = Action::ALL
        .into_iter()
        .map(|action| (action, allows(subject, action, &workspace)))
        .collect();
    let own = Facts {
        owns: true,
        target_is_self: true,
        ..workspace
    };
    let on_own = Action::ALL
        .into_iter()
        .filter(|action| action.contextual())
        .map(|action| (action, allows(subject, action, &own)))
        .collect();
    Some(CapabilitiesResponse {
        workspace_id,
        standing,
        role,
        actions,
        on_own,
        grantable_roles: grantable_roles(subject),
        link_invite_max_role: LINK_INVITE_MAX_ROLE,
        policy_revision: POLICY_REVISION,
    })
}

/// The generated matrix: every action for a fixed set of subjects and
/// fact variants, with its verdict. Written to
/// `src/generated/policy/policy-matrix.json` by the drift test
/// (`tests/ts_bindings.rs`); the web tests its local rules against it
/// (3.12), the revision test hashes it.
pub mod matrix {
    use std::collections::BTreeMap;
    use std::fmt::Write as _;

    use serde::Serialize;

    use super::{
        allows, Action, ActionKind, ActionScope, Fact, Facts, Subject, LINK_INVITE_MAX_ROLE,
        POLICY_REVISION,
    };
    use crate::auth::UserRole;
    use crate::workspace::WorkspaceMemberRole;

    /// The subjects of the matrix (`POLICY_PLAN.md` §6.3): workspace
    /// owner, admin, member and guest as platform users; operators and
    /// admins as members and guests; a platform guest who is a member; and
    /// people who are not members at all.
    pub const SUBJECTS: [(&str, UserRole, Option<WorkspaceMemberRole>); 13] = [
        ("O", UserRole::User, Some(WorkspaceMemberRole::Owner)),
        ("A", UserRole::User, Some(WorkspaceMemberRole::Admin)),
        ("M", UserRole::User, Some(WorkspaceMemberRole::Member)),
        ("G", UserRole::User, Some(WorkspaceMemberRole::Guest)),
        ("OpM", UserRole::Operator, Some(WorkspaceMemberRole::Member)),
        ("AdM", UserRole::Admin, Some(WorkspaceMemberRole::Member)),
        ("AdG", UserRole::Admin, Some(WorkspaceMemberRole::Guest)),
        ("OpG", UserRole::Operator, Some(WorkspaceMemberRole::Guest)),
        ("PgM", UserRole::Guest, Some(WorkspaceMemberRole::Member)),
        ("X", UserRole::User, None),
        ("OpX", UserRole::Operator, None),
        ("AdX", UserRole::Admin, None),
        ("PgX", UserRole::Guest, None),
    ];

    #[derive(Debug, Clone, Serialize)]
    pub struct ActionInfo {
        pub kind: ActionKind,
        pub scope: ActionScope,
        pub reads: &'static [Fact],
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct Case {
        pub subject: &'static str,
        pub action: Action,
        pub facts: Facts,
        pub allowed: bool,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct Matrix {
        pub revision: u32,
        pub link_invite_max_role: WorkspaceMemberRole,
        pub actions: BTreeMap<Action, ActionInfo>,
        pub subjects: BTreeMap<&'static str, Subject>,
        pub cases: Vec<Case>,
    }

    /// The fact variants an action is evaluated with.
    pub fn variants(action: Action) -> Vec<Facts> {
        let mut out = vec![Facts::NONE];
        let reads = action.reads();
        if reads.contains(&Fact::Owns) {
            out.push(Facts::owned(true));
        }
        match action {
            Action::InviteCreate => {
                for role in WorkspaceMemberRole::ALL {
                    out.push(Facts::NONE.granting(role));
                }
            }
            Action::MembersRemove => {
                for role in WorkspaceMemberRole::ALL {
                    out.push(Facts::target(Some(role), false));
                }
                out.push(Facts::target(None, true));
            }
            Action::MembersRoleChange => {
                for target in WorkspaceMemberRole::ALL {
                    for granted in WorkspaceMemberRole::ALL {
                        out.push(Facts::target(Some(target), false).granting(granted));
                    }
                }
                for granted in WorkspaceMemberRole::ALL {
                    out.push(Facts::target(None, true).granting(granted));
                }
            }
            Action::TeamsMemberRemove => out.push(Facts::target(None, true)),
            Action::TicketPlan => out.push(Facts::NONE.planning(true)),
            _ => {}
        }
        if action.scope() != ActionScope::Platform {
            out.push(Facts::unplaced());
        }
        out
    }

    pub fn build() -> Matrix {
        let actions = Action::ALL
            .into_iter()
            .map(|action| {
                (
                    action,
                    ActionInfo {
                        kind: action.kind(),
                        scope: action.scope(),
                        reads: action.reads(),
                    },
                )
            })
            .collect();
        let subjects = SUBJECTS
            .iter()
            .map(|(name, platform, membership)| {
                (*name, Subject::new(platform.clone(), *membership))
            })
            .collect();
        let mut cases = Vec::new();
        for (name, platform, membership) in &SUBJECTS {
            let subject = Subject::new(platform.clone(), *membership);
            for action in Action::ALL {
                for facts in variants(action) {
                    cases.push(Case {
                        subject: name,
                        action,
                        facts,
                        allowed: allows(&subject, action, &facts),
                    });
                }
            }
        }
        Matrix {
            revision: POLICY_REVISION,
            link_invite_max_role: LINK_INVITE_MAX_ROLE,
            actions,
            subjects,
            cases,
        }
    }

    /// The matrix as JSON: the header pretty-printed, one case per line,
    /// so a changed verdict is a one-line diff.
    pub fn to_json(matrix: &Matrix) -> Result<String, serde_json::Error> {
        let mut out = String::from("{\n");
        let _ = writeln!(out, "  \"revision\": {},", matrix.revision);
        let _ = writeln!(
            out,
            "  \"link_invite_max_role\": {},",
            serde_json::to_string(&matrix.link_invite_max_role)?
        );
        let _ = writeln!(out, "  \"actions\": {{");
        let mut first = true;
        for (action, info) in &matrix.actions {
            if !first {
                out.push_str(",\n");
            }
            first = false;
            let _ = write!(
                out,
                "    {}: {}",
                serde_json::to_string(action)?,
                serde_json::to_string(info)?
            );
        }
        out.push_str("\n  },\n  \"subjects\": {\n");
        first = true;
        for (name, subject) in &matrix.subjects {
            if !first {
                out.push_str(",\n");
            }
            first = false;
            let _ = write!(
                out,
                "    {}: {}",
                serde_json::to_string(name)?,
                serde_json::to_string(subject)?
            );
        }
        out.push_str("\n  },\n  \"cases\": [\n");
        out.push_str(&cases_json(matrix)?);
        out.push_str("\n  ]\n}\n");
        Ok(out)
    }

    /// The cases, one per line: what the revision digest covers.
    pub fn cases_json(matrix: &Matrix) -> Result<String, serde_json::Error> {
        let lines = matrix
            .cases
            .iter()
            .map(|case| serde_json::to_string(case).map(|line| format!("    {line}")))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(lines.join(",\n"))
    }

    /// FNV-1a (64 bit) of the cases: a cheap fingerprint of every verdict,
    /// not a security property.
    pub fn digest(matrix: &Matrix) -> Result<String, serde_json::Error> {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in cases_json(matrix)?.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Ok(format!("{hash:016x}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_action_is_listed_once_and_spelled_as_serde_spells_it() {
        let mut seen = std::collections::BTreeSet::new();
        for action in Action::ALL {
            assert!(seen.insert(action), "{action:?} twice in Action::ALL");
            assert_eq!(
                serde_json::to_value(action).expect("serialise"),
                serde_json::json!(action.as_str())
            );
            let back: Action =
                serde_json::from_value(serde_json::json!(action.as_str())).expect("parse");
            assert_eq!(back, action);
        }
        assert_eq!(seen.len(), 33);
    }

    #[test]
    fn standing_follows_the_one_rule() {
        let s = |p: UserRole, m: Option<WorkspaceMemberRole>| Subject::new(p, m);
        use WorkspaceMemberRole as R;
        assert_eq!(
            standing(&s(UserRole::User, Some(R::Guest)), false),
            Some(Standing::Reader)
        );
        assert_eq!(
            standing(&s(UserRole::Admin, Some(R::Guest)), false),
            Some(Standing::Reader)
        );
        assert_eq!(
            standing(&s(UserRole::User, Some(R::Owner)), false),
            Some(Standing::Manager)
        );
        assert_eq!(
            standing(&s(UserRole::User, Some(R::Admin)), false),
            Some(Standing::Manager)
        );
        assert_eq!(
            standing(&s(UserRole::User, Some(R::Member)), false),
            Some(Standing::Contributor)
        );
        assert_eq!(
            standing(&s(UserRole::Operator, Some(R::Member)), false),
            Some(Standing::Manager)
        );
        assert_eq!(standing(&s(UserRole::Admin, None), false), None);
        assert_eq!(
            standing(&s(UserRole::Admin, None), true),
            Some(Standing::Manager)
        );
        assert_eq!(standing(&s(UserRole::Operator, Some(R::Owner)), true), None);
    }
}
