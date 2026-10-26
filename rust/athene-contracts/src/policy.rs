//! The permission policy (roadmap step 3.8a, `POLICY_PLAN.md` §2; custom
//! roles phase A, `artifacts/roles/CONCEPT.md`): what a person may do in
//! Athene, as one table of actions over one set of permissions.
//!
//! Pure functions, no IO. The core loads the facts — the platform role from
//! the signed principal, the membership (tier and the role's permission
//! mask) from one row, ownership from the resource — and asks [`allows`];
//! `GET /work/workspaces/:id/capabilities` answers the same question for
//! every action at once ([`capabilities`]); the web holds the server's
//! answer and checks its loading fallback against the generated matrix
//! (`src/generated/policy/policy-matrix.json`, see [`matrix`]).
//!
//! - **Permissions** ([`Permission`]): the ticks of a role, about thirty,
//!   some as *own/any* pairs. A workspace's roles are bundles of them
//!   ([`PermissionSet`], a bit mask the database stores as `BIGINT`).
//! - **Actions** ([`Action`]): what the server checks and the web asks.
//!   Each one needs one permission, an own/any pair, or is governance
//!   ([`requires`], exhaustive: an action without a rule does not compile).
//! - **Tiers**: the membership's `role` column stays (`owner | admin |
//!   member | guest`). Owners and admins are fixed: every permission, and
//!   they alone govern ([`governs`], J1) — roles, role changes, removing
//!   people, the workspace's settings. A member's or guest's rights are
//!   their role's mask; a guest's are capped by [`GUEST_CEILING`] after
//!   everything else.
//! - A platform operator or admin who is a member keeps the content rights
//!   of a manager ([`PLATFORM_MEMBER_EXTRA`], E7) but does not govern.
//! - **Platform guests write nothing** (D3), wherever they are a member.
//! - **No escalation**: an invitation hands out only a role whose
//!   permissions are a subset of the inviter's own and whose tier is no
//!   higher ([`may_grant`]); everything else about roles is governance.
//!
//! Changing a verdict changes the matrix, and `tests/policy_matrix.rs` then
//! asks for [`POLICY_REVISION`] + 1, which the capabilities answer carries
//! so a client can tell its cached rules are stale.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use ts_rs::TS;
use utoipa::ToSchema;

use crate::athene_ai::AtheneAiAvailability;
use crate::auth::UserRole;
use crate::workspace::WorkspaceMemberRole;

/// The rules' revision. Bumped whenever a verdict in the matrix changes
/// (`tests/policy_matrix.rs` keeps the history).
pub const POLICY_REVISION: u32 = 6;

/// The most an open link may grant (J6). Admins and owners are invited by
/// email only: a forwarded link must not hand out the workspace.
pub const LINK_INVITE_MAX_ROLE: WorkspaceMemberRole = WorkspaceMemberRole::Member;

// ---------------------------------------------------------------------------
// Permissions
// ---------------------------------------------------------------------------

/// One tick of a role (`CONCEPT.md` §3). The wire names are what the API
/// and the web use; the bit ([`Permission::bit`]) is what the database
/// stores and never changes for a permission once shipped.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS,
)]
#[ts(export_to = "policy/")]
pub enum Permission {
    // Workspace
    #[serde(rename = "labels.manage")]
    LabelsManage,
    #[serde(rename = "ticket_types.manage")]
    TicketTypesManage,
    #[serde(rename = "fields.manage")]
    FieldsManage,
    #[serde(rename = "teams.manage")]
    TeamsManage,
    /// Create and edit events (UX9); delete one's own.
    #[serde(rename = "event.write")]
    EventWrite,
    /// Delete other people's events.
    #[serde(rename = "event.delete_any")]
    EventDeleteAny,
    // Projects
    #[serde(rename = "project.create")]
    ProjectCreate,
    /// Projects one created or leads.
    #[serde(rename = "project.edit_own")]
    ProjectEditOwn,
    #[serde(rename = "project.edit_any")]
    ProjectEditAny,
    // Tickets
    #[serde(rename = "ticket.create")]
    TicketCreate,
    /// Fields, status, assignee, links.
    #[serde(rename = "ticket.edit")]
    TicketEdit,
    /// Take a ticket from a queue (queues are phase C).
    #[serde(rename = "ticket.take")]
    TicketTake,
    /// Move a ticket to done or closed.
    #[serde(rename = "ticket.close")]
    TicketClose,
    /// Tickets one reported.
    #[serde(rename = "ticket.delete_own")]
    TicketDeleteOwn,
    #[serde(rename = "ticket.delete_any")]
    TicketDeleteAny,
    #[serde(rename = "ticket.check")]
    TicketCheck,
    #[serde(rename = "ticket.watch")]
    TicketWatch,
    #[serde(rename = "comment.create")]
    CommentCreate,
    /// Delete other people's comments.
    #[serde(rename = "comment.moderate")]
    CommentModerate,
    /// Log time; delete one's own entries.
    #[serde(rename = "time.log")]
    TimeLog,
    /// Edit or delete other people's time entries.
    #[serde(rename = "time.moderate")]
    TimeModerate,
    // Sprints & releases
    /// Put a ticket into a sprint or a release (replaces the workspace
    /// setting `ticket_planning`, E8).
    #[serde(rename = "ticket.plan")]
    TicketPlan,
    /// Sprints and releases themselves.
    #[serde(rename = "project.plan")]
    ProjectPlan,
    // Docs
    #[serde(rename = "doc.create")]
    DocCreate,
    #[serde(rename = "doc.write")]
    DocWrite,
    /// Archive one's own docs and collections.
    #[serde(rename = "doc.delete_own")]
    DocDeleteOwn,
    /// Archive anyone's docs, manage every collection.
    #[serde(rename = "doc.manage")]
    DocManage,
    // Members
    /// Invite people, into roles within one's own rights only.
    #[serde(rename = "invite.create")]
    InviteCreate,
    /// See, revoke and renew invitations (within one's own rights).
    #[serde(rename = "invite.manage")]
    InviteManage,
}

/// The sections of the role editor, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "policy/")]
pub enum PermissionGroup {
    Workspace,
    Projects,
    Tickets,
    Planning,
    Docs,
    Members,
}

impl Permission {
    /// Every permission, in the editor's order.
    pub const ALL: [Permission; 29] = [
        Permission::LabelsManage,
        Permission::TicketTypesManage,
        Permission::FieldsManage,
        Permission::TeamsManage,
        Permission::EventWrite,
        Permission::EventDeleteAny,
        Permission::ProjectCreate,
        Permission::ProjectEditOwn,
        Permission::ProjectEditAny,
        Permission::TicketCreate,
        Permission::TicketEdit,
        Permission::TicketTake,
        Permission::TicketClose,
        Permission::TicketDeleteOwn,
        Permission::TicketDeleteAny,
        Permission::TicketCheck,
        Permission::TicketWatch,
        Permission::CommentCreate,
        Permission::CommentModerate,
        Permission::TimeLog,
        Permission::TimeModerate,
        Permission::TicketPlan,
        Permission::ProjectPlan,
        Permission::DocCreate,
        Permission::DocWrite,
        Permission::DocDeleteOwn,
        Permission::DocManage,
        Permission::InviteCreate,
        Permission::InviteManage,
    ];

    /// The stored bit. Fixed forever: the database keeps masks of these,
    /// so a permission keeps its bit and a removed one leaves a hole.
    pub const fn bit(self) -> u32 {
        match self {
            Permission::LabelsManage => 0,
            Permission::TicketTypesManage => 1,
            Permission::FieldsManage => 2,
            Permission::TeamsManage => 3,
            Permission::EventWrite => 4,
            Permission::EventDeleteAny => 5,
            Permission::ProjectCreate => 6,
            Permission::ProjectEditOwn => 7,
            Permission::ProjectEditAny => 8,
            Permission::TicketCreate => 9,
            Permission::TicketEdit => 10,
            Permission::TicketTake => 11,
            Permission::TicketClose => 12,
            Permission::TicketDeleteOwn => 13,
            Permission::TicketDeleteAny => 14,
            Permission::TicketCheck => 15,
            Permission::TicketWatch => 16,
            Permission::CommentCreate => 17,
            Permission::CommentModerate => 18,
            Permission::TimeLog => 19,
            Permission::TimeModerate => 20,
            Permission::TicketPlan => 21,
            Permission::ProjectPlan => 22,
            Permission::DocCreate => 23,
            Permission::DocWrite => 24,
            Permission::DocDeleteOwn => 25,
            Permission::DocManage => 26,
            Permission::InviteCreate => 27,
            Permission::InviteManage => 28,
        }
    }

    /// The wire name (`ticket.edit`), the same string serde writes.
    pub const fn as_str(self) -> &'static str {
        match self {
            Permission::LabelsManage => "labels.manage",
            Permission::TicketTypesManage => "ticket_types.manage",
            Permission::FieldsManage => "fields.manage",
            Permission::TeamsManage => "teams.manage",
            Permission::EventWrite => "event.write",
            Permission::EventDeleteAny => "event.delete_any",
            Permission::ProjectCreate => "project.create",
            Permission::ProjectEditOwn => "project.edit_own",
            Permission::ProjectEditAny => "project.edit_any",
            Permission::TicketCreate => "ticket.create",
            Permission::TicketEdit => "ticket.edit",
            Permission::TicketTake => "ticket.take",
            Permission::TicketClose => "ticket.close",
            Permission::TicketDeleteOwn => "ticket.delete_own",
            Permission::TicketDeleteAny => "ticket.delete_any",
            Permission::TicketCheck => "ticket.check",
            Permission::TicketWatch => "ticket.watch",
            Permission::CommentCreate => "comment.create",
            Permission::CommentModerate => "comment.moderate",
            Permission::TimeLog => "time.log",
            Permission::TimeModerate => "time.moderate",
            Permission::TicketPlan => "ticket.plan",
            Permission::ProjectPlan => "project.plan",
            Permission::DocCreate => "doc.create",
            Permission::DocWrite => "doc.write",
            Permission::DocDeleteOwn => "doc.delete_own",
            Permission::DocManage => "doc.manage",
            Permission::InviteCreate => "invite.create",
            Permission::InviteManage => "invite.manage",
        }
    }

    pub fn parse(name: &str) -> Option<Permission> {
        Permission::ALL.into_iter().find(|p| p.as_str() == name)
    }

    pub const fn group(self) -> PermissionGroup {
        match self {
            Permission::LabelsManage
            | Permission::TicketTypesManage
            | Permission::FieldsManage
            | Permission::TeamsManage
            | Permission::EventWrite
            | Permission::EventDeleteAny => PermissionGroup::Workspace,
            Permission::ProjectCreate | Permission::ProjectEditOwn | Permission::ProjectEditAny => {
                PermissionGroup::Projects
            }
            Permission::TicketCreate
            | Permission::TicketEdit
            | Permission::TicketTake
            | Permission::TicketClose
            | Permission::TicketDeleteOwn
            | Permission::TicketDeleteAny
            | Permission::TicketCheck
            | Permission::TicketWatch
            | Permission::CommentCreate
            | Permission::CommentModerate
            | Permission::TimeLog
            | Permission::TimeModerate => PermissionGroup::Tickets,
            Permission::TicketPlan | Permission::ProjectPlan => PermissionGroup::Planning,
            Permission::DocCreate
            | Permission::DocWrite
            | Permission::DocDeleteOwn
            | Permission::DocManage => PermissionGroup::Docs,
            Permission::InviteCreate | Permission::InviteManage => PermissionGroup::Members,
        }
    }

    /// Whether a guest role may hold it ([`GUEST_CEILING`]).
    pub const fn guest_allowed(self) -> bool {
        GUEST_CEILING.contains(self)
    }

    /// Whether the permission does anything yet. Every one does since the
    /// queues (phase C1) made `ticket.take` count; kept so a later
    /// permission can again be shown as coming.
    pub const fn effective_yet(self) -> bool {
        true
    }
}

/// A role's permissions as a bit mask over [`Permission::bit`]. On the wire
/// it is the list of permission names (in [`Permission::ALL`] order); in
/// the database a `BIGINT`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, ToSchema)]
#[schema(value_type = Vec<Permission>)]
pub struct PermissionSet(u64);

impl PermissionSet {
    pub const EMPTY: PermissionSet = PermissionSet(0);
    /// Every permission there is.
    pub const ALL: PermissionSet = PermissionSet::of(&Permission::ALL);

    pub const fn of(permissions: &[Permission]) -> PermissionSet {
        let mut bits = 0u64;
        let mut i = 0;
        while i < permissions.len() {
            bits |= 1u64 << permissions[i].bit();
            i += 1;
        }
        PermissionSet(bits)
    }

    /// From the stored mask; unknown bits (a permission removed since, or
    /// a newer binary's) are dropped, never granted.
    pub const fn from_bits(bits: i64) -> PermissionSet {
        // A mask never uses the sign bit (bits stay below 63).
        PermissionSet(bits.cast_unsigned() & PermissionSet::ALL.0)
    }

    /// The stored mask.
    pub const fn bits(self) -> i64 {
        self.0.cast_signed()
    }

    pub const fn contains(self, permission: Permission) -> bool {
        self.0 & (1u64 << permission.bit()) != 0
    }

    pub const fn with(self, permission: Permission) -> PermissionSet {
        PermissionSet(self.0 | (1u64 << permission.bit()))
    }

    pub const fn without(self, permission: Permission) -> PermissionSet {
        PermissionSet(self.0 & !(1u64 << permission.bit()))
    }

    pub const fn union(self, other: PermissionSet) -> PermissionSet {
        PermissionSet(self.0 | other.0)
    }

    pub const fn intersection(self, other: PermissionSet) -> PermissionSet {
        PermissionSet(self.0 & other.0)
    }

    pub const fn difference(self, other: PermissionSet) -> PermissionSet {
        PermissionSet(self.0 & !other.0)
    }

    pub const fn is_subset(self, other: PermissionSet) -> bool {
        self.0 & !other.0 == 0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn intersects(self, other: PermissionSet) -> bool {
        self.0 & other.0 != 0
    }

    /// The permissions in [`Permission::ALL`] order.
    pub fn iter(self) -> impl Iterator<Item = Permission> {
        Permission::ALL
            .into_iter()
            .filter(move |p| self.contains(*p))
    }

    pub fn to_vec(self) -> Vec<Permission> {
        self.iter().collect()
    }

    pub fn len(self) -> usize {
        self.iter().count()
    }
}

impl FromIterator<Permission> for PermissionSet {
    fn from_iter<I: IntoIterator<Item = Permission>>(iter: I) -> Self {
        iter.into_iter()
            .fold(PermissionSet::EMPTY, PermissionSet::with)
    }
}

impl Serialize for PermissionSet {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter())
    }
}

impl<'de> Deserialize<'de> for PermissionSet {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Vec::<Permission>::deserialize(deserializer)?
            .into_iter()
            .collect())
    }
}

/// What a guest role can ever hold: watching and commenting (reading is
/// membership). Applied after everything else, overrides included (§2.4).
pub const GUEST_CEILING: PermissionSet =
    PermissionSet::of(&[Permission::TicketWatch, Permission::CommentCreate]);

/// The content rights a platform operator or admin keeps as a member of
/// any role of tier `member` (E7, J1): everything but governance, which is
/// the workspace's own owners' and admins' — inviting included.
pub const PLATFORM_MEMBER_EXTRA: PermissionSet = PermissionSet::ALL
    .without(Permission::InviteCreate)
    .without(Permission::InviteManage);

/// The permissions that make a role read as "manager" in the derived
/// [`Standing`] (a display, no rule reads it).
pub const MANAGEMENT: PermissionSet = PermissionSet::of(&[
    Permission::LabelsManage,
    Permission::TicketTypesManage,
    Permission::FieldsManage,
    Permission::TeamsManage,
    Permission::EventDeleteAny,
    Permission::ProjectEditAny,
    Permission::TicketDeleteAny,
    Permission::CommentModerate,
    Permission::TimeModerate,
    Permission::ProjectPlan,
    Permission::DocManage,
]);

/// The system role "Mitglied" of a new workspace: exactly the contributor
/// of before, without creating projects (E5). A migrated workspace's
/// "Mitglied" has `ticket.plan` only if its `ticket_planning` was not
/// `managers` (E8, migration 0032).
pub const MEMBER_DEFAULT: PermissionSet = PermissionSet::of(&[
    Permission::EventWrite,
    Permission::ProjectEditOwn,
    Permission::TicketCreate,
    Permission::TicketEdit,
    Permission::TicketTake,
    Permission::TicketClose,
    Permission::TicketDeleteOwn,
    Permission::TicketCheck,
    Permission::TicketWatch,
    Permission::CommentCreate,
    Permission::TimeLog,
    Permission::TicketPlan,
    Permission::DocCreate,
    Permission::DocWrite,
    Permission::DocDeleteOwn,
]);

/// The system role "Gast": read, watch, comment.
pub const GUEST_DEFAULT: PermissionSet = GUEST_CEILING;

/// The roles a workspace starts with or can add from a template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "policy/")]
pub enum RolePreset {
    Developer,
    Tester,
    ProductOwner,
}

impl RolePreset {
    pub const ALL: [RolePreset; 3] = [
        RolePreset::Developer,
        RolePreset::Tester,
        RolePreset::ProductOwner,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            RolePreset::Developer => "developer",
            RolePreset::Tester => "tester",
            RolePreset::ProductOwner => "product_owner",
        }
    }

    pub fn parse(key: &str) -> Option<RolePreset> {
        RolePreset::ALL.into_iter().find(|p| p.as_str() == key)
    }

    /// The template's ticks (`CONCEPT.md` §2.2, §3).
    pub const fn permissions(self) -> PermissionSet {
        match self {
            // Tickets: create, edit, take, plan; time; docs. No projects.
            RolePreset::Developer => PermissionSet::of(&[
                Permission::EventWrite,
                Permission::ProjectEditOwn,
                Permission::TicketCreate,
                Permission::TicketEdit,
                Permission::TicketTake,
                Permission::TicketClose,
                Permission::TicketDeleteOwn,
                Permission::TicketCheck,
                Permission::TicketWatch,
                Permission::CommentCreate,
                Permission::TimeLog,
                Permission::TicketPlan,
                Permission::DocCreate,
                Permission::DocWrite,
                Permission::DocDeleteOwn,
            ]),
            // Tickets: create, edit, take, close; comment; docs read. No
            // projects, no sprints.
            RolePreset::Tester => PermissionSet::of(&[
                Permission::EventWrite,
                Permission::TicketCreate,
                Permission::TicketEdit,
                Permission::TicketTake,
                Permission::TicketClose,
                Permission::TicketDeleteOwn,
                Permission::TicketCheck,
                Permission::TicketWatch,
                Permission::CommentCreate,
                Permission::TimeLog,
            ]),
            // Developer plus projects, planning, the workspace's labels,
            // types and fields, and deleting anyone's tickets.
            RolePreset::ProductOwner => PLATFORM_MEMBER_EXTRA,
        }
    }

    /// The template's default name (German, like the system roles).
    pub const fn default_name(self) -> &'static str {
        match self {
            RolePreset::Developer => "Developer",
            RolePreset::Tester => "Tester",
            RolePreset::ProductOwner => "Product Owner",
        }
    }

    /// A palette token (`TicketTypeColor`).
    pub const fn default_color(self) -> &'static str {
        match self {
            RolePreset::Developer => "blue",
            RolePreset::Tester => "green",
            RolePreset::ProductOwner => "purple",
        }
    }

    /// A lucide icon name.
    pub const fn default_icon(self) -> &'static str {
        match self {
            RolePreset::Developer => "code",
            RolePreset::Tester => "flask-conical",
            RolePreset::ProductOwner => "compass",
        }
    }
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

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
    /// tickets, sprints, releases, reports, docs, labels, roles.
    #[serde(rename = "workspace.read")]
    WorkspaceRead,
    /// Name, description, colour and settings.
    #[serde(rename = "workspace.settings.edit")]
    WorkspaceSettingsEdit,
    /// Create, edit, reorder and delete the workspace's roles.
    #[serde(rename = "roles.manage")]
    RolesManage,
    /// Facts: `granted_role`, `granted_grants` (none = any the actor may
    /// grant).
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
    /// Write a page or file it; also create a collection.
    #[serde(rename = "doc.create")]
    DocCreate,
    /// Edit a page, and the collections' names and order.
    #[serde(rename = "doc.write")]
    DocWrite,
    /// Facts: `owns` (the author).
    #[serde(rename = "doc.delete")]
    DocDelete,
    /// Delete a collection. Facts: `owns` (its creator).
    #[serde(rename = "doc.collection.manage")]
    DocCollectionManage,
    #[serde(rename = "project.create")]
    ProjectCreate,
    /// Facts: `owns` (creator or lead).
    #[serde(rename = "project.edit")]
    ProjectEdit,
    /// Sprints and releases themselves: create, edit, start, complete,
    /// delete.
    #[serde(rename = "project.plan")]
    ProjectPlan,
    /// Put a ticket into a sprint or a release, or take it out.
    #[serde(rename = "ticket.plan")]
    TicketPlan,
    #[serde(rename = "ticket.create")]
    TicketCreate,
    /// Edit, move, assign, triage, label, link.
    #[serde(rename = "ticket.edit")]
    TicketEdit,
    /// Hang an existing field onto one ticket, or take it off (custom
    /// fields CF1, E3). The tick "Tickets bearbeiten".
    #[serde(rename = "field.attach")]
    FieldAttach,
    /// Suggest a field for the ticket's team, type or project (E3). The
    /// tick "Tickets bearbeiten".
    #[serde(rename = "field.suggest")]
    FieldSuggest,
    /// Facts: `owns` (the reporter).
    #[serde(rename = "ticket.delete")]
    TicketDelete,
    /// Take a ticket from a queue (phase C).
    #[serde(rename = "ticket.take")]
    TicketTake,
    /// Move a ticket to done or closed.
    #[serde(rename = "ticket.close")]
    TicketClose,
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
    /// Put an event ("Termin") into the workspace's calendar (UX9).
    #[serde(rename = "event.create")]
    EventCreate,
    /// Change an event: title, time, description, participants, links.
    #[serde(rename = "event.edit")]
    EventEdit,
    /// Facts: `owns` (the creator).
    #[serde(rename = "event.delete")]
    EventDelete,
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
    /// The tier the person acted on holds now.
    TargetRole,
    /// The tier an invitation or a role change would hand out.
    GrantedRole,
    /// The permissions of the role an invitation would hand out.
    GrantedGrants,
}

impl Action {
    /// Every action, in wire order.
    pub const ALL: [Action; 43] = [
        Action::AdminAccess,
        Action::WorkspaceCreate,
        Action::SlaRead,
        Action::SlaManage,
        Action::WorkspaceRead,
        Action::WorkspaceSettingsEdit,
        Action::RolesManage,
        Action::InviteCreate,
        Action::InviteManage,
        Action::MembersRoleChange,
        Action::MembersRemove,
        Action::TeamsManage,
        Action::TeamsMemberRemove,
        Action::LabelsManage,
        Action::TicketTypesManage,
        Action::FieldsManage,
        Action::DocCreate,
        Action::DocWrite,
        Action::DocDelete,
        Action::DocCollectionManage,
        Action::ProjectCreate,
        Action::ProjectEdit,
        Action::ProjectPlan,
        Action::TicketCreate,
        Action::TicketEdit,
        Action::TicketDelete,
        Action::TicketTake,
        Action::TicketClose,
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
        Action::EventCreate,
        Action::EventEdit,
        Action::EventDelete,
        Action::FieldAttach,
        Action::FieldSuggest,
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
            Action::RolesManage => "roles.manage",
            Action::InviteCreate => "invite.create",
            Action::InviteManage => "invite.manage",
            Action::MembersRoleChange => "members.role.change",
            Action::MembersRemove => "members.remove",
            Action::TeamsManage => "teams.manage",
            Action::TeamsMemberRemove => "teams.member.remove",
            Action::LabelsManage => "labels.manage",
            Action::TicketTypesManage => "ticket_types.manage",
            Action::FieldsManage => "fields.manage",
            Action::DocCreate => "doc.create",
            Action::DocWrite => "doc.write",
            Action::DocDelete => "doc.delete",
            Action::DocCollectionManage => "doc.collection.manage",
            Action::ProjectCreate => "project.create",
            Action::ProjectEdit => "project.edit",
            Action::ProjectPlan => "project.plan",
            Action::TicketCreate => "ticket.create",
            Action::TicketEdit => "ticket.edit",
            Action::TicketDelete => "ticket.delete",
            Action::TicketTake => "ticket.take",
            Action::TicketClose => "ticket.close",
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
            Action::EventCreate => "event.create",
            Action::EventEdit => "event.edit",
            Action::EventDelete => "event.delete",
            Action::FieldAttach => "field.attach",
            Action::FieldSuggest => "field.suggest",
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
            | Action::FieldAttach
            | Action::FieldSuggest
            | Action::TicketPlan
            | Action::TicketDelete
            | Action::TicketTake
            | Action::TicketClose
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
            Action::InviteCreate => &[Fact::GrantedRole, Fact::GrantedGrants],
            Action::MembersRoleChange => &[Fact::TargetRole, Fact::GrantedRole, Fact::TargetIsSelf],
            Action::MembersRemove => &[Fact::TargetRole, Fact::TargetIsSelf],
            Action::TeamsMemberRemove => &[Fact::TargetIsSelf],
            Action::DocDelete
            | Action::DocCollectionManage
            | Action::ProjectEdit
            | Action::TicketDelete
            | Action::CommentEdit
            | Action::CommentDelete
            | Action::TimeDelete
            | Action::EventDelete => &[Fact::Owns],
            _ => &[],
        }
    }

    /// Whether the verdict depends on the resource or the person acted on
    /// (see [`Action::reads`]).
    pub const fn contextual(self) -> bool {
        !self.reads().is_empty()
    }
}

/// One side of an own/any rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    /// Whoever is a member.
    Always,
    Never,
    Needs(Permission),
}

/// The governance rules, which stay code, not ticks (§2.3, E4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Governance {
    /// The workspace's name, description, colour and settings; its roles.
    Runs,
    /// Give someone a role: never above one's own tier, owners only by
    /// owners, one's own only downwards (J7).
    RoleChange,
    /// Remove someone (owners only by owners); leaving is everyone's.
    Remove,
}

/// What an action needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Need {
    /// Decided on the platform role alone.
    Platform,
    /// Every member of the workspace.
    Member,
    One(Permission),
    /// On the actor's own resource, on anybody's.
    OwnOrAny {
        own: Gate,
        any: Gate,
    },
    /// The actor themselves always, anyone else with the permission.
    SelfOr(Permission),
    /// The permission, and only for roles within the actor's own rights
    /// ([`may_grant`]).
    Grant(Permission),
    Governance(Governance),
}

impl Need {
    /// The permissions the rule can read (for the editor's "controls" hint
    /// and the denial log).
    pub fn permissions(self) -> Vec<Permission> {
        let gate = |g: Gate| match g {
            Gate::Needs(p) => Some(p),
            Gate::Always | Gate::Never => None,
        };
        match self {
            Need::One(p) | Need::SelfOr(p) | Need::Grant(p) => vec![p],
            Need::OwnOrAny { own, any } => gate(own).into_iter().chain(gate(any)).collect(),
            Need::Platform | Need::Member | Need::Governance(_) => Vec::new(),
        }
    }
}

/// The rule table: which permission each action needs. Exhaustive, so a
/// new action without a rule does not compile.
pub const fn requires(action: Action) -> Need {
    use Permission as P;
    match action {
        Action::AdminAccess | Action::WorkspaceCreate | Action::SlaRead | Action::SlaManage => {
            Need::Platform
        }
        Action::WorkspaceRead => Need::Member,
        Action::WorkspaceSettingsEdit | Action::RolesManage => Need::Governance(Governance::Runs),
        Action::MembersRoleChange => Need::Governance(Governance::RoleChange),
        Action::MembersRemove => Need::Governance(Governance::Remove),
        Action::InviteCreate => Need::Grant(P::InviteCreate),
        Action::InviteManage => Need::One(P::InviteManage),
        Action::TeamsManage => Need::One(P::TeamsManage),
        Action::TeamsMemberRemove => Need::SelfOr(P::TeamsManage),
        Action::LabelsManage => Need::One(P::LabelsManage),
        Action::TicketTypesManage => Need::One(P::TicketTypesManage),
        Action::FieldsManage => Need::One(P::FieldsManage),
        Action::DocCreate => Need::One(P::DocCreate),
        Action::DocWrite => Need::One(P::DocWrite),
        Action::DocDelete | Action::DocCollectionManage => Need::OwnOrAny {
            own: Gate::Needs(P::DocDeleteOwn),
            any: Gate::Needs(P::DocManage),
        },
        Action::ProjectCreate => Need::One(P::ProjectCreate),
        Action::ProjectEdit => Need::OwnOrAny {
            own: Gate::Needs(P::ProjectEditOwn),
            any: Gate::Needs(P::ProjectEditAny),
        },
        Action::ProjectPlan => Need::One(P::ProjectPlan),
        Action::TicketPlan => Need::One(P::TicketPlan),
        Action::TicketCreate => Need::One(P::TicketCreate),
        Action::TicketEdit | Action::FieldAttach | Action::FieldSuggest => Need::One(P::TicketEdit),
        Action::TicketDelete => Need::OwnOrAny {
            own: Gate::Needs(P::TicketDeleteOwn),
            any: Gate::Needs(P::TicketDeleteAny),
        },
        Action::TicketTake => Need::One(P::TicketTake),
        Action::TicketClose => Need::One(P::TicketClose),
        Action::TicketWatch => Need::One(P::TicketWatch),
        Action::CommentCreate => Need::One(P::CommentCreate),
        // Your words are yours, at every tier.
        Action::CommentEdit => Need::OwnOrAny {
            own: Gate::Always,
            any: Gate::Never,
        },
        Action::CommentDelete => Need::OwnOrAny {
            own: Gate::Always,
            any: Gate::Needs(P::CommentModerate),
        },
        Action::CommentModerate => Need::One(P::CommentModerate),
        Action::TimeLog => Need::One(P::TimeLog),
        Action::TimeDelete => Need::OwnOrAny {
            own: Gate::Needs(P::TimeLog),
            any: Gate::Needs(P::TimeModerate),
        },
        Action::TimeModerate => Need::One(P::TimeModerate),
        Action::TicketCheck => Need::One(P::TicketCheck),
        Action::EventCreate | Action::EventEdit => Need::One(P::EventWrite),
        Action::EventDelete => Need::OwnOrAny {
            own: Gate::Needs(P::EventWrite),
            any: Gate::Needs(P::EventDeleteAny),
        },
    }
}

// ---------------------------------------------------------------------------
// Subjects
// ---------------------------------------------------------------------------

/// What someone may do in one workspace, weakest first. Derived for display
/// ([`standing`]); no rule reads it any more.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "policy/")]
pub enum Standing {
    /// Reads (and comments).
    Reader,
    /// Files and changes tickets.
    Contributor,
    /// Plans, moderates or sets the workspace's content up.
    Manager,
}

/// A membership as the policy reads it: the tier (`role` column) and the
/// permissions of the member's role (ignored for owners and admins, who
/// hold every one).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Membership {
    pub tier: WorkspaceMemberRole,
    pub grants: PermissionSet,
}

impl Membership {
    pub const fn new(tier: WorkspaceMemberRole, grants: PermissionSet) -> Self {
        Self { tier, grants }
    }

    /// An owner's or admin's membership (their mask is irrelevant), or a
    /// member's or guest's with the system role's defaults.
    pub const fn of_tier(tier: WorkspaceMemberRole) -> Self {
        Self {
            tier,
            grants: match tier {
                WorkspaceMemberRole::Owner | WorkspaceMemberRole::Admin => PermissionSet::ALL,
                WorkspaceMemberRole::Member => MEMBER_DEFAULT,
                WorkspaceMemberRole::Guest => GUEST_DEFAULT,
            },
        }
    }
}

/// Who is asking: the platform role (from the signed principal) and the
/// membership in the workspace in question, if any.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subject {
    pub platform_role: UserRole,
    /// `None`: not a member (or the resource belongs to no workspace).
    pub membership: Option<Membership>,
}

impl Subject {
    pub fn new(platform_role: UserRole, membership: Option<Membership>) -> Self {
        Self {
            platform_role,
            membership,
        }
    }

    pub fn tier(&self) -> Option<WorkspaceMemberRole> {
        self.membership.map(|m| m.tier)
    }

    /// The permissions in effect in the workspace: everything for owners
    /// and admins; the role's mask for a member, plus
    /// [`PLATFORM_MEMBER_EXTRA`] for a platform operator or admin; the
    /// role's mask capped by [`GUEST_CEILING`] for a guest; nothing for a
    /// stranger. A platform guest's writes are refused before this is read.
    pub fn effective(&self) -> PermissionSet {
        let Some(m) = self.membership else {
            return PermissionSet::EMPTY;
        };
        match m.tier {
            WorkspaceMemberRole::Owner | WorkspaceMemberRole::Admin => PermissionSet::ALL,
            WorkspaceMemberRole::Member
                if matches!(self.platform_role, UserRole::Operator | UserRole::Admin) =>
            {
                m.grants.union(PLATFORM_MEMBER_EXTRA)
            }
            WorkspaceMemberRole::Member => m.grants,
            WorkspaceMemberRole::Guest => m.grants.intersection(GUEST_CEILING),
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
    /// The tier the person acted on holds; `None` with `target_is_self`
    /// means the actor's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_role: Option<WorkspaceMemberRole>,
    /// The tier being handed out; `None` = any the actor may grant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granted_role: Option<WorkspaceMemberRole>,
    /// The permissions of the role being handed out. Required whenever
    /// `granted_role` is named for someone who does not govern: without
    /// it, nothing is handed out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granted_grants: Option<PermissionSet>,
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
        granted_grants: None,
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

    /// Acting on a person: their tier now and whether it is the actor.
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

    /// Handing out a role: its tier and its permissions.
    pub const fn granting_role(mut self, role: WorkspaceMemberRole, grants: PermissionSet) -> Self {
        self.granted_role = Some(role);
        self.granted_grants = Some(grants);
        self
    }
}

/// The derived standing, for display (no rule reads it):
///
/// - `unplaced`: nobody can be a member of "no workspace", so only the
///   platform admin, the one authority that is not workspace-scoped, gets
///   `Manager`; everyone else `None`.
/// - not a member: `None` — the caller answers `403`, exactly as for an id
///   that does not exist.
/// - otherwise `Manager` for owners and admins and whoever holds a
///   management permission, `Contributor` with `ticket.edit`, else
///   `Reader`.
pub fn standing(subject: &Subject, unplaced: bool) -> Option<Standing> {
    if unplaced {
        return matches!(subject.platform_role, UserRole::Admin).then_some(Standing::Manager);
    }
    let tier = subject.tier()?;
    let grants = subject.effective();
    Some(
        if matches!(
            tier,
            WorkspaceMemberRole::Owner | WorkspaceMemberRole::Admin
        ) || grants.intersects(MANAGEMENT)
        {
            Standing::Manager
        } else if grants.contains(Permission::TicketEdit) {
            Standing::Contributor
        } else {
            Standing::Reader
        },
    )
}

/// Whether the subject runs the workspace: its own owners and admins, not
/// platform operators or admins who are members (J1).
pub fn governs(subject: &Subject) -> bool {
    matches!(
        subject.tier(),
        Some(WorkspaceMemberRole::Owner | WorkspaceMemberRole::Admin)
    )
}

/// The verdict.
pub fn allows(subject: &Subject, action: Action, facts: &Facts) -> bool {
    // A platform guest changes nothing, wherever they are a member (D3).
    if action.kind() == ActionKind::Write && subject.platform_role == UserRole::Guest {
        return false;
    }
    let need = requires(action);
    if need == Need::Platform {
        return match action {
            Action::AdminAccess => matches!(subject.platform_role, UserRole::Admin),
            Action::SlaRead | Action::SlaManage => {
                matches!(subject.platform_role, UserRole::Operator | UserRole::Admin)
            }
            // Everyone but a platform guest (refused above).
            _ => true,
        };
    }
    // Data of no workspace: the platform admin's alone, as a manager who
    // does not govern (there is nobody to govern).
    let (grants, governing) = if facts.unplaced {
        if subject.platform_role != UserRole::Admin {
            return false;
        }
        (PermissionSet::ALL, false)
    } else {
        if subject.membership.is_none() {
            return false;
        }
        (subject.effective(), governs(subject))
    };
    let gate = |g: Gate| match g {
        Gate::Always => true,
        Gate::Never => false,
        Gate::Needs(p) => grants.contains(p),
    };
    match need {
        Need::Platform => false,
        Need::Member => true,
        Need::One(p) => grants.contains(p),
        Need::OwnOrAny { own, any } => gate(any) || (facts.owns && gate(own)),
        Need::SelfOr(p) => facts.target_is_self || grants.contains(p),
        Need::Grant(p) => {
            grants.contains(p)
                && within_own(subject, facts.granted_role)
                && (governing || grant_within(grants, facts))
        }
        Need::Governance(Governance::Runs) => governing,
        Need::Governance(Governance::RoleChange) => governing && changes_role(subject, facts),
        // Leaving is everyone's own call; removing others is governing.
        Need::Governance(Governance::Remove) => {
            facts.target_is_self || (governing && may_touch(subject, facts.target_role))
        }
    }
}

/// For someone who does not govern: a role named is handed out only with
/// its permissions known and within the actor's own.
fn grant_within(own: PermissionSet, facts: &Facts) -> bool {
    match (facts.granted_role, facts.granted_grants) {
        (None, None) => true,
        (_, Some(grants)) => grants.is_subset(own),
        (Some(_), None) => false,
    }
}

/// `members.role.change`: an owner's role, and the owner role, only by an
/// owner; never above the actor's own tier; one's own only downwards (J7).
/// The last owner is the repository's to protect (409).
fn changes_role(subject: &Subject, facts: &Facts) -> bool {
    if !may_touch(subject, facts.target_role) || !within_own(subject, facts.granted_role) {
        return false;
    }
    if facts.target_is_self {
        let current = facts.target_role.or(subject.tier());
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
    target != Some(WorkspaceMemberRole::Owner) || subject.tier() == Some(WorkspaceMemberRole::Owner)
}

/// The granted tier is no more than the subject's own (none named: fine).
fn within_own(subject: &Subject, granted: Option<WorkspaceMemberRole>) -> bool {
    match (granted, subject.tier()) {
        (None, _) => true,
        (Some(granted), Some(own)) => granted.rank() <= own.rank(),
        (Some(_), None) => false,
    }
}

/// Whether the subject may hand a role of `tier` with `grants` out by
/// invitation (the escalation rule, §7): at most their own tier, and for
/// someone who does not govern only permissions they hold themselves.
pub fn may_grant(subject: &Subject, tier: WorkspaceMemberRole, grants: PermissionSet) -> bool {
    allows(
        subject,
        Action::InviteCreate,
        &Facts::NONE.granting_role(tier, grants),
    )
}

/// The tiers the subject may hand out by invitation or role change, lowest
/// first; empty for whoever may do neither.
pub fn grantable_roles(subject: &Subject) -> Vec<WorkspaceMemberRole> {
    let Some(own) = subject.tier() else {
        return Vec::new();
    };
    if !allows(subject, Action::InviteCreate, &Facts::NONE)
        && !allows(subject, Action::MembersRoleChange, &Facts::NONE)
    {
        return Vec::new();
    }
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

/// Who work may be assigned to (J8, D7; UX-021): a member of its
/// workspace who is not a guest — a guest reads, comments and watches, but
/// cannot move or close what would be theirs — and never a platform guest,
/// who changes nothing. Work outside any workspace has no roster: anyone
/// but a platform guest.
pub fn assignable(assignee: &Subject, unplaced: bool) -> bool {
    assignee.platform_role != UserRole::Guest
        && (unplaced
            || assignee
                .membership
                .is_some_and(|m| m.tier != WorkspaceMemberRole::Guest))
}

/// Where a permission in a role's preview comes from ("Vorschau als
/// Rolle", §8.2), and each step of its chain (phase B, §2.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "policy/")]
pub enum PermissionSource {
    /// The role's tick (on or off).
    Role,
    /// Owner and admin hold everything; nothing changes that.
    Fixed,
    /// Off for every guest, whatever the role or an override says.
    GuestCeiling,
    /// An override for the role at a place (a project or a docs area).
    RoleOverride,
    /// An override for one of the member's teams at a place (the
    /// workspace, a project or a docs area).
    TeamOverride,
}

/// One step of a permission's chain: who said what, where.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "policy/")]
pub struct PermissionStep {
    pub source: PermissionSource,
    /// What this step says (allow or deny).
    pub allowed: bool,
    /// Where (overrides only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub place: Option<crate::overrides::OverridePlace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub place_id: Option<String>,
    /// The team (team overrides only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub team_id: Option<String>,
}

/// One line of the preview: the verdict, the step that decided it, and
/// every step on the way (the role's tick first).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "policy/")]
pub struct EffectivePermission {
    pub permission: Permission,
    pub allowed: bool,
    /// The deciding step's source.
    pub source: PermissionSource,
    /// The deciding step's place, if an override decided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub place: Option<crate::overrides::OverridePlace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub place_id: Option<String>,
    /// The deciding team, if a team override decided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub team_id: Option<String>,
    /// Every step that said something, in the order of the resolution.
    #[serde(default)]
    pub chain: Vec<PermissionStep>,
}

/// A role's effective permissions with their origin, for an ordinary user
/// holding it, without overrides (the platform operator's extra is not a
/// role's, §2.2). With overrides: [`crate::overrides::resolve_traced`].
pub fn effective_permissions(
    tier: WorkspaceMemberRole,
    grants: PermissionSet,
) -> Vec<EffectivePermission> {
    crate::overrides::resolve_traced(tier, grants, &[])
}

// ---------------------------------------------------------------------------
// Capabilities
// ---------------------------------------------------------------------------

/// The caller's role, as the capabilities name it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "policy/")]
pub struct CapabilitiesRole {
    /// `null` only where a membership has no role row (memory mode).
    pub id: Option<String>,
    pub name: String,
    /// A palette token (`TicketTypeColor`); `null` = neutral.
    pub color: Option<String>,
    /// `owner`, `admin`, `member`, `guest` for the system roles.
    pub system_key: Option<WorkspaceMemberRole>,
}

/// `GET /work/workspaces/:workspace_id/capabilities`: the caller's verdict
/// on every action in one workspace, in `{ "data": … }`.
///
/// Answered with `Cache-Control: no-store`; a membership or role change
/// counts from the next request (`roles_epoch` moves with every write to
/// the workspace's roles), a platform role change within the identity
/// cache's 30 s.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "policy/")]
pub struct CapabilitiesResponse {
    pub workspace_id: String,
    /// The place the verdicts are for (`?project_id=` /
    /// `?doc_collection_id=`; the workspace without either): the
    /// overrides there are applied (phase B).
    pub place: crate::overrides::OverridePlace,
    /// The project's or docs area's id; the workspace's for `workspace`.
    pub place_id: String,
    /// Derived for display; no rule reads it.
    pub standing: Standing,
    /// The membership's tier.
    pub tier: WorkspaceMemberRole,
    /// The caller's role in the workspace.
    pub role: CapabilitiesRole,
    /// The permissions in effect for the caller here (guest ceiling and
    /// the platform operator's extra applied).
    #[ts(as = "Vec<Permission>")]
    #[schema(value_type = Vec<Permission>)]
    pub permissions: PermissionSet,
    /// Moves with every write to the workspace's roles and their
    /// assignment; a client holding another epoch reloads.
    pub roles_epoch: i64,
    /// Every action. A contextual one is `true` when it is allowed on any
    /// resource (somebody else's, no role named); the platform actions are
    /// included.
    #[schema(value_type = BTreeMap<String, bool>)]
    pub actions: BTreeMap<Action, bool>,
    /// Contextual actions only: allowed on the caller's own resources (or
    /// on themselves). `can(a, ctx) = actions[a] || (on_own[a] && isOwn(ctx))`.
    #[schema(value_type = BTreeMap<String, bool>)]
    pub on_own: BTreeMap<Action, bool>,
    /// The tiers for `invite.create` and `members.role.change`, lowest
    /// first. Which roles exactly: `grantable` on each role of
    /// `GET /workspaces/:id/roles`.
    pub grantable_roles: Vec<WorkspaceMemberRole>,
    /// The most an open link may grant (J6).
    pub link_invite_max_role: WorkspaceMemberRole,
    /// The rules' revision (`POLICY_REVISION`); a client holding rules of
    /// another revision reloads them.
    pub policy_revision: u32,
    /// Whether Athene AI is there for the caller here, as far as the
    /// platform goes (`athene_ai`). Not a policy rule: the core fills it in
    /// from the platform's visibility and the workspace's beta enrollment.
    pub athene_ai: AtheneAiAvailability,
}

/// The capabilities of a member; `None` for a non-member (the endpoint
/// answers `403`, as for a workspace that does not exist).
pub fn capabilities(
    subject: &Subject,
    workspace_id: String,
    role: CapabilitiesRole,
    roles_epoch: i64,
) -> Option<CapabilitiesResponse> {
    let tier = subject.tier()?;
    let standing = standing(subject, false)?;
    let actions = Action::ALL
        .into_iter()
        .map(|action| (action, allows(subject, action, &Facts::NONE)))
        .collect();
    let own = Facts {
        owns: true,
        target_is_self: true,
        ..Facts::NONE
    };
    let on_own = Action::ALL
        .into_iter()
        .filter(|action| action.contextual())
        .map(|action| (action, allows(subject, action, &own)))
        .collect();
    Some(CapabilitiesResponse {
        place: crate::overrides::OverridePlace::Workspace,
        place_id: workspace_id.clone(),
        workspace_id,
        standing,
        tier,
        role,
        permissions: subject.effective(),
        roles_epoch,
        actions,
        on_own,
        grantable_roles: grantable_roles(subject),
        link_invite_max_role: LINK_INVITE_MAX_ROLE,
        policy_revision: POLICY_REVISION,
        athene_ai: AtheneAiAvailability::HIDDEN,
    })
}

/// The generated matrix: every action for a fixed set of subjects and
/// fact variants, with its verdict. Written to
/// `src/generated/policy/policy-matrix.json` by the drift test
/// (`tests/ts_bindings.rs`); the web tests its loading fallback against it
/// (3.12), the revision test hashes it.
pub mod matrix {
    use std::collections::BTreeMap;
    use std::fmt::Write as _;

    use serde::Serialize;

    use super::{
        allows, Action, ActionKind, ActionScope, Fact, Facts, Membership, Permission,
        PermissionSet, RolePreset, Subject, GUEST_CEILING, LINK_INVITE_MAX_ROLE, MEMBER_DEFAULT,
        POLICY_REVISION,
    };
    use crate::auth::UserRole;
    use crate::workspace::WorkspaceMemberRole;

    /// A custom role of the matrix: it may invite, edits tickets and
    /// comments, and nothing else.
    pub const CUSTOM: PermissionSet = PermissionSet::of(&[
        Permission::InviteCreate,
        Permission::InviteManage,
        Permission::TicketEdit,
        Permission::TicketWatch,
        Permission::CommentCreate,
    ]);

    /// A guest role whose stored mask asks for writing: the ceiling cuts it.
    pub const GUEST_WITH_WRITES: PermissionSet = GUEST_CEILING
        .with(Permission::TicketEdit)
        .with(Permission::TicketCreate)
        .with(Permission::InviteCreate);

    /// The subjects of the matrix (`POLICY_PLAN.md` §6.3, `CONCEPT.md`
    /// §5): owner, admin, the system roles "Mitglied" and "Gast" as
    /// platform users; operators and admins as members and guests; a
    /// platform guest who is a member; the presets, a custom role, an
    /// empty role and a guest role asking for writes; and people who are
    /// not members at all.
    pub fn subjects() -> Vec<(&'static str, Subject)> {
        use WorkspaceMemberRole as R;
        let m = |tier, grants| Some(Membership::new(tier, grants));
        let fixed = |tier| Some(Membership::of_tier(tier));
        vec![
            ("O", Subject::new(UserRole::User, fixed(R::Owner))),
            ("A", Subject::new(UserRole::User, fixed(R::Admin))),
            ("M", Subject::new(UserRole::User, fixed(R::Member))),
            ("G", Subject::new(UserRole::User, fixed(R::Guest))),
            ("OpM", Subject::new(UserRole::Operator, fixed(R::Member))),
            ("AdM", Subject::new(UserRole::Admin, fixed(R::Member))),
            ("AdG", Subject::new(UserRole::Admin, fixed(R::Guest))),
            ("OpG", Subject::new(UserRole::Operator, fixed(R::Guest))),
            ("PgM", Subject::new(UserRole::Guest, fixed(R::Member))),
            (
                "Dev",
                Subject::new(
                    UserRole::User,
                    m(R::Member, RolePreset::Developer.permissions()),
                ),
            ),
            (
                "Tst",
                Subject::new(
                    UserRole::User,
                    m(R::Member, RolePreset::Tester.permissions()),
                ),
            ),
            (
                "PO",
                Subject::new(
                    UserRole::User,
                    m(R::Member, RolePreset::ProductOwner.permissions()),
                ),
            ),
            ("Cus", Subject::new(UserRole::User, m(R::Member, CUSTOM))),
            (
                "Nil",
                Subject::new(UserRole::User, m(R::Member, PermissionSet::EMPTY)),
            ),
            (
                "GsW",
                Subject::new(UserRole::User, m(R::Guest, GUEST_WITH_WRITES)),
            ),
            ("X", Subject::new(UserRole::User, None)),
            ("OpX", Subject::new(UserRole::Operator, None)),
            ("AdX", Subject::new(UserRole::Admin, None)),
            ("PgX", Subject::new(UserRole::Guest, None)),
        ]
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct ActionInfo {
        pub kind: ActionKind,
        pub scope: ActionScope,
        pub reads: &'static [Fact],
        pub permissions: Vec<Permission>,
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
                // Into the presets, a custom role, an empty one and every
                // permission there is.
                let masks = [
                    MEMBER_DEFAULT,
                    RolePreset::Developer.permissions(),
                    RolePreset::Tester.permissions(),
                    RolePreset::ProductOwner.permissions(),
                    CUSTOM,
                    PermissionSet::EMPTY,
                    PermissionSet::ALL,
                ];
                for mask in masks {
                    out.push(Facts::NONE.granting_role(WorkspaceMemberRole::Member, mask));
                }
                out.push(Facts::NONE.granting_role(WorkspaceMemberRole::Guest, GUEST_CEILING));
                out.push(Facts::NONE.granting_role(WorkspaceMemberRole::Admin, PermissionSet::ALL));
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
                        permissions: super::requires(action).permissions(),
                    },
                )
            })
            .collect();
        let all = subjects();
        let mut cases = Vec::new();
        for (name, subject) in &all {
            for action in Action::ALL {
                for facts in variants(action) {
                    cases.push(Case {
                        subject: name,
                        action,
                        facts,
                        allowed: allows(subject, action, &facts),
                    });
                }
            }
        }
        Matrix {
            revision: POLICY_REVISION,
            link_invite_max_role: LINK_INVITE_MAX_ROLE,
            actions,
            subjects: all.into_iter().collect(),
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
        assert_eq!(seen.len(), 43);
    }

    #[test]
    fn every_permission_has_its_own_bit_and_name() {
        let mut bits = std::collections::BTreeSet::new();
        let mut names = std::collections::BTreeSet::new();
        for p in Permission::ALL {
            assert!(bits.insert(p.bit()), "{p:?}: bit {} twice", p.bit());
            assert!(names.insert(p.as_str()), "{p:?}: name twice");
            assert!(p.bit() < 63, "{p:?}: the sign bit stays free");
            assert_eq!(
                serde_json::to_value(p).expect("serialise"),
                serde_json::json!(p.as_str())
            );
            assert_eq!(Permission::parse(p.as_str()), Some(p));
        }
        assert_eq!(PermissionSet::ALL.len(), Permission::ALL.len());
    }

    #[test]
    fn a_set_round_trips_as_names_and_as_bits() {
        let set = PermissionSet::of(&[Permission::TicketEdit, Permission::LabelsManage]);
        let json = serde_json::to_value(set).expect("serialise");
        assert_eq!(json, serde_json::json!(["labels.manage", "ticket.edit"]));
        let back: PermissionSet = serde_json::from_value(json).expect("parse");
        assert_eq!(back, set);
        assert_eq!(PermissionSet::from_bits(set.bits()), set);
        // Bits nobody knows are dropped, never granted.
        assert_eq!(
            PermissionSet::from_bits(set.bits() | (1 << 50)),
            set,
            "an unknown bit"
        );
        assert_eq!(PermissionSet::from_bits(-1), PermissionSet::ALL);
        assert!(serde_json::from_value::<PermissionSet>(serde_json::json!(["root"])).is_err());
    }

    #[test]
    fn every_permission_is_read_by_some_action() {
        let read: PermissionSet = Action::ALL
            .into_iter()
            .flat_map(|a| requires(a).permissions())
            .collect();
        assert_eq!(read, PermissionSet::ALL);
    }

    #[test]
    fn the_presets_and_system_roles_stay_inside_their_bounds() {
        assert!(GUEST_DEFAULT.is_subset(GUEST_CEILING));
        assert!(!MEMBER_DEFAULT.contains(Permission::ProjectCreate), "E5");
        for preset in RolePreset::ALL {
            let p = preset.permissions();
            assert!(!p.contains(Permission::InviteCreate), "{preset:?} invites");
            assert_eq!(RolePreset::parse(preset.as_str()), Some(preset));
        }
        assert!(!RolePreset::Developer
            .permissions()
            .contains(Permission::ProjectCreate));
        assert!(RolePreset::ProductOwner
            .permissions()
            .contains(Permission::ProjectCreate));
        assert!(!RolePreset::Tester
            .permissions()
            .contains(Permission::TicketPlan));
        assert!(RolePreset::Developer
            .permissions()
            .is_subset(RolePreset::ProductOwner.permissions()));
    }

    #[test]
    fn standing_is_derived_from_the_permissions() {
        let s = |p: UserRole, m: Option<Membership>| Subject::new(p, m);
        use WorkspaceMemberRole as R;
        let of = |r| Some(Membership::of_tier(r));
        assert_eq!(
            standing(&s(UserRole::User, of(R::Guest)), false),
            Some(Standing::Reader)
        );
        assert_eq!(
            standing(&s(UserRole::Admin, of(R::Guest)), false),
            Some(Standing::Reader)
        );
        assert_eq!(
            standing(&s(UserRole::User, of(R::Owner)), false),
            Some(Standing::Manager)
        );
        assert_eq!(
            standing(&s(UserRole::User, of(R::Member)), false),
            Some(Standing::Contributor)
        );
        assert_eq!(
            standing(&s(UserRole::Operator, of(R::Member)), false),
            Some(Standing::Manager)
        );
        assert_eq!(
            standing(
                &s(
                    UserRole::User,
                    Some(Membership::new(
                        R::Member,
                        RolePreset::ProductOwner.permissions()
                    ))
                ),
                false
            ),
            Some(Standing::Manager)
        );
        assert_eq!(
            standing(
                &s(
                    UserRole::User,
                    Some(Membership::new(R::Member, PermissionSet::EMPTY))
                ),
                false
            ),
            Some(Standing::Reader)
        );
        assert_eq!(standing(&s(UserRole::Admin, None), false), None);
        assert_eq!(
            standing(&s(UserRole::Admin, None), true),
            Some(Standing::Manager)
        );
        assert_eq!(standing(&s(UserRole::Operator, of(R::Owner)), true), None);
    }
}
