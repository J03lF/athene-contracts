//! Permission overrides (custom roles, phase B; `artifacts/roles/
//! CONCEPT.md` §2.4, §3, §5, §8.3): at a **place** — a project, a docs
//! area, or the workspace itself — a **target** — a role or a team —
//! says *allow*, *deny* or *inherit* per permission, the way Discord's
//! channel overrides do.
//!
//! **Resolution** ([`resolve`], and [`resolve_traced`] with the source
//! chain for the preview), for a member at a place:
//!
//! 1. Owner and admin: everything, always; no override reaches them.
//! 2. The role's ticks are the base.
//! 3. Then each level of the place's path, general to specific —
//!    workspace, then project, then docs area:
//!    - the role's override there replaces the value (`v = (v & !deny) |
//!      allow`), and
//!    - then the member's teams' overrides there do; between teams on the
//!      same level *allow wins* (E3). So at one place the team beats the
//!      role, and a more specific place beats a more general one.
//!    - *Inherit* says nothing; the value comes from the level above, at
//!      last from the role's tick.
//! 4. A guest is cut to [`GUEST_CEILING`] after everything. A platform
//!    guest's writes are refused by [`crate::policy::allows`] before any
//!    of this is read; a platform operator's or admin's extra as a member
//!    (`PLATFORM_MEMBER_EXTRA`, E7) is added after this, as a fixed mask.
//!
//! **Which permission is overridable where** ([`OverridePlace::overridable`],
//! §3 column "Ü"): a project takes the project, ticket, time and planning
//! rights; a docs area the docs rights; the workspace only team targets
//! (a role's workspace-wide value is its tick) and every right but
//! inviting, watching and commenting. Inviting stays a role's tick only.
//!
//! The function is pure: the core resolves with masks from an epoch-keyed
//! cache (no statement on the hot path); the web runs the TypeScript port
//! (`src/payloads/overrides.ts`) for its "inherited" hints, and both are
//! held to the same cases (`src/generated/policy/override-cases.json`,
//! [`cases`]).
//!
//! - `GET /workspaces/:workspace_id/overrides` — every member: the
//!   workspace's overrides (a few dozen rows), with the epoch.
//! - `PUT /workspaces/:workspace_id/overrides` — `roles.manage` (owners and
//!   admins): set one target's override at one place; both lists empty
//!   clears it. With `version` (3.7: `409` when it changed meanwhile).

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::policy::{
    EffectivePermission, Permission, PermissionSet, PermissionSource, PermissionStep, GUEST_CEILING,
};
use crate::workspace::WorkspaceMemberRole;

// ---------------------------------------------------------------------------
// Vocabulary
// ---------------------------------------------------------------------------

/// Where an override hangs.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "policy/")]
pub enum OverridePlace {
    /// The whole workspace (team targets only).
    Workspace,
    Project,
    /// A docs area (collection).
    DocCollection,
}

/// Whom an override names.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "policy/")]
pub enum OverrideTarget {
    Role,
    Team,
}

/// One permission's setting in an override (the tri-state control:
/// *Verbieten · Erben · Erlauben*).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "policy/")]
pub enum OverrideState {
    Allow,
    Inherit,
    Deny,
}

/// What a project's override may name: projects, tickets, time, planning.
pub const PROJECT_OVERRIDABLE: PermissionSet = PermissionSet::of(&[
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
]);

/// What a docs area's override may name.
pub const DOC_COLLECTION_OVERRIDABLE: PermissionSet = PermissionSet::of(&[
    Permission::DocCreate,
    Permission::DocWrite,
    Permission::DocDeleteOwn,
    Permission::DocManage,
]);

/// What a team's workspace-wide override may name: every right but
/// inviting (a role's tick only) and watching and commenting (per project
/// only, §3).
pub const WORKSPACE_OVERRIDABLE: PermissionSet = PermissionSet::ALL
    .without(Permission::InviteCreate)
    .without(Permission::InviteManage)
    .without(Permission::TicketWatch)
    .without(Permission::CommentCreate);

impl OverridePlace {
    pub const ALL: [OverridePlace; 3] = [
        OverridePlace::Workspace,
        OverridePlace::Project,
        OverridePlace::DocCollection,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            OverridePlace::Workspace => "workspace",
            OverridePlace::Project => "project",
            OverridePlace::DocCollection => "doc_collection",
        }
    }

    pub fn parse(raw: &str) -> Option<OverridePlace> {
        OverridePlace::ALL.into_iter().find(|p| p.as_str() == raw)
    }

    /// The permissions an override here may name.
    pub const fn overridable(self) -> PermissionSet {
        match self {
            OverridePlace::Workspace => WORKSPACE_OVERRIDABLE,
            OverridePlace::Project => PROJECT_OVERRIDABLE,
            OverridePlace::DocCollection => DOC_COLLECTION_OVERRIDABLE,
        }
    }

    /// Whether `target` may hang here: a role's workspace-wide value is
    /// its tick, so the workspace takes teams only.
    pub const fn takes(self, target: OverrideTarget) -> bool {
        !matches!(
            (self, target),
            (OverridePlace::Workspace, OverrideTarget::Role)
        )
    }
}

impl OverrideTarget {
    pub const fn as_str(self) -> &'static str {
        match self {
            OverrideTarget::Role => "role",
            OverrideTarget::Team => "team",
        }
    }

    pub fn parse(raw: &str) -> Option<OverrideTarget> {
        [OverrideTarget::Role, OverrideTarget::Team]
            .into_iter()
            .find(|t| t.as_str() == raw)
    }
}

impl Permission {
    /// Whether an override at `place` may name the permission.
    pub const fn overridable_at(self, place: OverridePlace) -> bool {
        place.overridable().contains(self)
    }
}

// ---------------------------------------------------------------------------
// Masks and resolution
// ---------------------------------------------------------------------------

/// One override as two disjoint masks; a permission in neither inherits.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OverrideMasks {
    pub allow: PermissionSet,
    pub deny: PermissionSet,
}

impl OverrideMasks {
    pub const NONE: OverrideMasks = OverrideMasks {
        allow: PermissionSet::EMPTY,
        deny: PermissionSet::EMPTY,
    };

    /// `None` when a permission is both allowed and denied.
    pub const fn new(allow: PermissionSet, deny: PermissionSet) -> Option<OverrideMasks> {
        if allow.intersects(deny) {
            None
        } else {
            Some(OverrideMasks { allow, deny })
        }
    }

    /// From the stored masks; a permission somehow in both is denied
    /// (the database's check forbids it — this is the safe reading).
    pub const fn from_bits(allow: i64, deny: i64) -> OverrideMasks {
        let deny = PermissionSet::from_bits(deny);
        OverrideMasks {
            allow: PermissionSet::from_bits(allow).difference(deny),
            deny,
        }
    }

    pub const fn state(self, permission: Permission) -> OverrideState {
        if self.allow.contains(permission) {
            OverrideState::Allow
        } else if self.deny.contains(permission) {
            OverrideState::Deny
        } else {
            OverrideState::Inherit
        }
    }

    pub const fn is_empty(self) -> bool {
        self.allow.is_empty() && self.deny.is_empty()
    }

    /// Everything the override names.
    pub const fn named(self) -> PermissionSet {
        self.allow.union(self.deny)
    }

    /// The level's word on `value`.
    pub const fn apply(self, value: PermissionSet) -> PermissionSet {
        value.difference(self.deny).union(self.allow)
    }

    /// Several teams' overrides on the same level as one: allow wins
    /// (E3), so a permission one team allows is not denied by another.
    pub const fn join(self, other: OverrideMasks) -> OverrideMasks {
        let allow = self.allow.union(other.allow);
        OverrideMasks {
            allow,
            deny: self.deny.union(other.deny).difference(allow),
        }
    }
}

/// One level of a place's path as the hot path holds it: the role's
/// override there and the member's teams' overrides there, joined.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LevelMasks {
    pub role: OverrideMasks,
    pub teams: OverrideMasks,
}

/// The permissions in effect for a member of `tier` whose role ticks
/// `base`, through `levels` (general to specific). Pure bit operations.
pub fn resolve(
    tier: WorkspaceMemberRole,
    base: PermissionSet,
    levels: &[LevelMasks],
) -> PermissionSet {
    if matches!(
        tier,
        WorkspaceMemberRole::Owner | WorkspaceMemberRole::Admin
    ) {
        return PermissionSet::ALL;
    }
    let value = levels
        .iter()
        .fold(base, |v, level| level.teams.apply(level.role.apply(v)));
    match tier {
        WorkspaceMemberRole::Guest => value.intersection(GUEST_CEILING),
        _ => value,
    }
}

/// A team's override at one level, for the traced resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "policy/")]
pub struct TeamOverrideMasks {
    pub team_id: String,
    #[ts(as = "Vec<Permission>")]
    #[schema(value_type = Vec<Permission>)]
    pub allow: PermissionSet,
    #[ts(as = "Vec<Permission>")]
    #[schema(value_type = Vec<Permission>)]
    pub deny: PermissionSet,
}

/// One level of a place's path with who said what, for the preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "policy/")]
pub struct OverrideLevel {
    pub place: OverridePlace,
    pub place_id: String,
    /// The role's override here (empty: none).
    #[ts(as = "Vec<Permission>")]
    #[schema(value_type = Vec<Permission>)]
    pub role_allow: PermissionSet,
    #[ts(as = "Vec<Permission>")]
    #[schema(value_type = Vec<Permission>)]
    pub role_deny: PermissionSet,
    /// The member's teams' overrides here (none: no team says anything).
    pub teams: Vec<TeamOverrideMasks>,
}

impl OverrideLevel {
    /// The level as the hot path holds it.
    pub fn masks(&self) -> LevelMasks {
        LevelMasks {
            role: OverrideMasks::from_bits(self.role_allow.bits(), self.role_deny.bits()),
            teams: self.teams.iter().fold(OverrideMasks::NONE, |all, t| {
                all.join(OverrideMasks::from_bits(t.allow.bits(), t.deny.bits()))
            }),
        }
    }
}

/// [`resolve`] with the source chain of each permission, in
/// [`Permission::ALL`] order (the preview, §8.2).
pub fn resolve_traced(
    tier: WorkspaceMemberRole,
    base: PermissionSet,
    levels: &[OverrideLevel],
) -> Vec<EffectivePermission> {
    Permission::ALL
        .into_iter()
        .map(|permission| trace(tier, base, levels, permission))
        .collect()
}

fn step(
    source: PermissionSource,
    allowed: bool,
    level: Option<&OverrideLevel>,
    team_id: Option<&str>,
) -> PermissionStep {
    PermissionStep {
        source,
        allowed,
        place: level.map(|l| l.place),
        place_id: level.map(|l| l.place_id.clone()),
        team_id: team_id.map(str::to_string),
    }
}

fn trace(
    tier: WorkspaceMemberRole,
    base: PermissionSet,
    levels: &[OverrideLevel],
    permission: Permission,
) -> EffectivePermission {
    if matches!(
        tier,
        WorkspaceMemberRole::Owner | WorkspaceMemberRole::Admin
    ) {
        let fixed = step(PermissionSource::Fixed, true, None, None);
        return EffectivePermission {
            permission,
            allowed: true,
            source: PermissionSource::Fixed,
            place: None,
            place_id: None,
            team_id: None,
            chain: vec![fixed],
        };
    }
    let mut chain = vec![step(
        PermissionSource::Role,
        base.contains(permission),
        None,
        None,
    )];
    let mut decided = 0usize;
    for level in levels {
        let role = OverrideMasks::from_bits(level.role_allow.bits(), level.role_deny.bits());
        match role.state(permission) {
            OverrideState::Inherit => {}
            state => {
                chain.push(step(
                    PermissionSource::RoleOverride,
                    state == OverrideState::Allow,
                    Some(level),
                    None,
                ));
                decided = chain.len() - 1;
            }
        }
        // Every team that says something is on the chain; the first one
        // that allows decides (allow wins), else the first that denies.
        let mut allowing = None;
        let mut denying = None;
        for team in &level.teams {
            match OverrideMasks::from_bits(team.allow.bits(), team.deny.bits()).state(permission) {
                OverrideState::Inherit => {}
                state => {
                    let allowed = state == OverrideState::Allow;
                    chain.push(step(
                        PermissionSource::TeamOverride,
                        allowed,
                        Some(level),
                        Some(&team.team_id),
                    ));
                    let at = chain.len() - 1;
                    if allowed {
                        allowing.get_or_insert(at);
                    } else {
                        denying.get_or_insert(at);
                    }
                }
            }
        }
        if let Some(at) = allowing.or(denying) {
            decided = at;
        }
    }
    let mut allowed = chain[decided].allowed;
    let mut source = chain[decided].source;
    let (mut place, mut place_id, mut team_id) = (
        chain[decided].place,
        chain[decided].place_id.clone(),
        chain[decided].team_id.clone(),
    );
    if tier == WorkspaceMemberRole::Guest && !GUEST_CEILING.contains(permission) {
        chain.push(step(PermissionSource::GuestCeiling, false, None, None));
        allowed = false;
        source = PermissionSource::GuestCeiling;
        place = None;
        place_id = None;
        team_id = None;
    }
    EffectivePermission {
        permission,
        allowed,
        source,
        place,
        place_id,
        team_id,
        chain,
    }
}

// ---------------------------------------------------------------------------
// Wire
// ---------------------------------------------------------------------------

/// One override.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "roles/")]
pub struct PermissionOverrideResponse {
    pub id: String,
    pub workspace_id: String,
    pub place: OverridePlace,
    /// The project's or docs area's id; the workspace's for `workspace`.
    pub place_id: String,
    pub target: OverrideTarget,
    pub target_id: String,
    #[ts(as = "Vec<Permission>")]
    #[schema(value_type = Vec<Permission>)]
    pub allow: PermissionSet,
    #[ts(as = "Vec<Permission>")]
    #[schema(value_type = Vec<Permission>)]
    pub deny: PermissionSet,
    /// For `PUT` (3.7).
    pub version: i64,
    pub updated_at: String,
}

/// `GET /workspaces/:workspace_id/overrides`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "roles/")]
pub struct WorkspaceOverridesResponse {
    pub overrides: Vec<PermissionOverrideResponse>,
    pub roles_epoch: i64,
    /// Whether the caller may change them (`roles.manage`).
    pub editable: bool,
}

/// `PUT /workspaces/:workspace_id/overrides`: one target's override at one
/// place. Both lists empty clears it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "roles/")]
pub struct SetOverrideRequest {
    pub place: OverridePlace,
    /// The project's or docs area's id; the workspace's for `workspace`.
    pub place_id: String,
    pub target: OverrideTarget,
    pub target_id: String,
    #[serde(default)]
    #[ts(as = "Vec<Permission>")]
    #[schema(value_type = Vec<Permission>)]
    pub allow: PermissionSet,
    #[serde(default)]
    #[ts(as = "Vec<Permission>")]
    #[schema(value_type = Vec<Permission>)]
    pub deny: PermissionSet,
    /// The version the edit started from; `0` when there was none.
    pub version: i64,
}

/// `PUT /workspaces/:workspace_id/overrides`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "roles/")]
pub struct SetOverrideResponse {
    /// `null`: cleared (or there was nothing to clear).
    #[serde(rename = "override")]
    #[ts(rename = "override")]
    pub entry: Option<PermissionOverrideResponse>,
    pub roles_epoch: i64,
}

/// `GET /work/workspaces/:workspace_id/capabilities` and
/// `GET /workspaces/:workspace_id/roles/:role_id/effective`: the place to
/// answer for (at most one of project and docs area; none: the
/// workspace). The preview also takes a team.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "policy/")]
pub struct PlaceQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub doc_collection_id: Option<String>,
    /// The preview only: resolve as a member of this team.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub team_id: Option<String>,
}

// ---------------------------------------------------------------------------
// The shared cases
// ---------------------------------------------------------------------------

/// The cases Rust and the web are both held to
/// (`src/generated/policy/override-cases.json`): the six worked examples of
/// the concept with their expected verdicts, and a grid of every setting
/// of one permission over two levels with a role and two teams.
pub mod cases {
    use serde::Serialize;

    use super::{OverrideLevel, OverridePlace, TeamOverrideMasks};
    use crate::policy::{Permission, PermissionSet, PermissionSource, RolePreset, MEMBER_DEFAULT};
    use crate::workspace::WorkspaceMemberRole;

    /// One case: who, which levels, and what must come out.
    #[derive(Debug, Clone, Serialize)]
    pub struct Case {
        pub name: String,
        pub tier: WorkspaceMemberRole,
        pub base: PermissionSet,
        pub levels: Vec<OverrideLevel>,
        pub permission: Permission,
        pub allowed: bool,
        pub source: PermissionSource,
        /// The deciding team, if a team decided.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub team_id: Option<String>,
    }

    fn level(
        place: OverridePlace,
        place_id: &str,
        role: (PermissionSet, PermissionSet),
        teams: &[(&str, PermissionSet, PermissionSet)],
    ) -> OverrideLevel {
        OverrideLevel {
            place,
            place_id: place_id.to_string(),
            role_allow: role.0,
            role_deny: role.1,
            teams: teams
                .iter()
                .map(|(id, allow, deny)| TeamOverrideMasks {
                    team_id: (*id).to_string(),
                    allow: *allow,
                    deny: *deny,
                })
                .collect(),
        }
    }

    const NO: PermissionSet = PermissionSet::EMPTY;

    fn one(p: Permission) -> PermissionSet {
        PermissionSet::EMPTY.with(p)
    }

    /// A case whose expectation is written by hand: the resolver must
    /// agree (`tests/policy_matrix.rs`), or the generator panics.
    #[allow(clippy::too_many_arguments)]
    fn example(
        name: &str,
        tier: WorkspaceMemberRole,
        base: PermissionSet,
        levels: Vec<OverrideLevel>,
        permission: Permission,
        allowed: bool,
        source: PermissionSource,
        team_id: Option<&str>,
    ) -> Case {
        Case {
            name: name.to_string(),
            tier,
            base,
            levels,
            permission,
            allowed,
            source,
            team_id: team_id.map(str::to_string),
        }
    }

    /// The concept's six examples (§2.4), each in its variants.
    pub fn examples() -> Vec<Case> {
        use OverridePlace as Pl;
        use Permission as P;
        use PermissionSource as S;
        use WorkspaceMemberRole as R;
        let ws = |teams: &[(&str, PermissionSet, PermissionSet)]| {
            level(Pl::Workspace, "ws", (NO, NO), teams)
        };
        let dev = RolePreset::Developer.permissions();
        let po = RolePreset::ProductOwner.permissions();
        let tester = RolePreset::Tester.permissions();
        vec![
            // 1: a workspace right is the role's tick; below the workspace
            // nothing names it.
            example(
                "1 developer creates no project",
                R::Member,
                dev,
                vec![],
                P::ProjectCreate,
                false,
                S::Role,
                None,
            ),
            example(
                "1 product owner creates projects",
                R::Member,
                po,
                vec![],
                P::ProjectCreate,
                true,
                S::Role,
                None,
            ),
            // 2: Mitglied may create docs; area Handbuch denies Mitglied,
            // allows team Doku.
            example(
                "2 member in team Doku writes in Handbuch",
                R::Member,
                MEMBER_DEFAULT,
                vec![
                    ws(&[]),
                    level(
                        Pl::DocCollection,
                        "handbuch",
                        (NO, one(P::DocCreate)),
                        &[("doku", one(P::DocCreate), NO)],
                    ),
                ],
                P::DocCreate,
                true,
                S::TeamOverride,
                Some("doku"),
            ),
            example(
                "2 member outside Doku does not in Handbuch",
                R::Member,
                MEMBER_DEFAULT,
                vec![
                    ws(&[]),
                    level(Pl::DocCollection, "handbuch", (NO, one(P::DocCreate)), &[]),
                ],
                P::DocCreate,
                false,
                S::RoleOverride,
                None,
            ),
            example(
                "2 member elsewhere creates docs",
                R::Member,
                MEMBER_DEFAULT,
                vec![ws(&[]), level(Pl::DocCollection, "anderer", (NO, NO), &[])],
                P::DocCreate,
                true,
                S::Role,
                None,
            ),
            // 3: project Kunde X denies Tester "Tickets bearbeiten".
            example(
                "3 tester edits nothing in Kunde X",
                R::Member,
                tester,
                vec![
                    ws(&[]),
                    level(Pl::Project, "kunde-x", (NO, one(P::TicketEdit)), &[]),
                ],
                P::TicketEdit,
                false,
                S::RoleOverride,
                None,
            ),
            example(
                "3 tester still comments in Kunde X",
                R::Member,
                tester,
                vec![
                    ws(&[]),
                    level(Pl::Project, "kunde-x", (NO, one(P::TicketEdit)), &[]),
                ],
                P::CommentCreate,
                true,
                S::Role,
                None,
            ),
            example(
                "3 tester edits in other projects",
                R::Member,
                tester,
                vec![ws(&[]), level(Pl::Project, "other", (NO, NO), &[])],
                P::TicketEdit,
                true,
                S::Role,
                None,
            ),
            // 4: as 3, and team QA-Kunde-X allows at the project; Anna is
            // in it: the team beats the role at the same place.
            example(
                "4 Anna in QA-Kunde-X edits in Kunde X",
                R::Member,
                tester,
                vec![
                    ws(&[]),
                    level(
                        Pl::Project,
                        "kunde-x",
                        (NO, one(P::TicketEdit)),
                        &[("qa-kunde-x", one(P::TicketEdit), NO)],
                    ),
                ],
                P::TicketEdit,
                true,
                S::TeamOverride,
                Some("qa-kunde-x"),
            ),
            // 5: Release-Crew allows "Sprints planen" workspace-wide;
            // project Legacy denies the role Developer.
            example(
                "5 crew developer plans in the workspace",
                R::Member,
                dev,
                vec![ws(&[("release-crew", one(P::ProjectPlan), NO)])],
                P::ProjectPlan,
                true,
                S::TeamOverride,
                Some("release-crew"),
            ),
            example(
                "5 crew developer does not plan in Legacy",
                R::Member,
                dev,
                vec![
                    ws(&[("release-crew", one(P::ProjectPlan), NO)]),
                    level(Pl::Project, "legacy", (NO, one(P::ProjectPlan)), &[]),
                ],
                P::ProjectPlan,
                false,
                S::RoleOverride,
                None,
            ),
            example(
                "5 crew developer plans in Legacy once the crew is allowed there too",
                R::Member,
                dev,
                vec![
                    ws(&[("release-crew", one(P::ProjectPlan), NO)]),
                    level(
                        Pl::Project,
                        "legacy",
                        (NO, one(P::ProjectPlan)),
                        &[("release-crew", one(P::ProjectPlan), NO)],
                    ),
                ],
                P::ProjectPlan,
                true,
                S::TeamOverride,
                Some("release-crew"),
            ),
            // 6: a guest role allowed "Tickets bearbeiten": the ceiling cuts.
            example(
                "6 guest stays a guest",
                R::Guest,
                crate::policy::GUEST_DEFAULT,
                vec![
                    ws(&[("everyone", one(P::TicketEdit), NO)]),
                    level(Pl::Project, "p", (one(P::TicketEdit), NO), &[]),
                ],
                P::TicketEdit,
                false,
                S::GuestCeiling,
                None,
            ),
            // Allow wins between teams on the same level (E3).
            example(
                "E3 allow wins between teams",
                R::Member,
                tester,
                vec![
                    ws(&[]),
                    level(
                        Pl::Project,
                        "p",
                        (NO, NO),
                        &[("a", NO, one(P::TicketEdit)), ("b", one(P::TicketEdit), NO)],
                    ),
                ],
                P::TicketEdit,
                true,
                S::TeamOverride,
                Some("b"),
            ),
            // Owner and admin are out of reach.
            example(
                "owner is never denied",
                R::Owner,
                NO,
                vec![
                    ws(&[("t", NO, one(P::TicketEdit))]),
                    level(Pl::Project, "p", (NO, one(P::TicketEdit)), &[]),
                ],
                P::TicketEdit,
                true,
                S::Fixed,
                None,
            ),
            example(
                "admin is never denied",
                R::Admin,
                NO,
                vec![level(
                    Pl::Project,
                    "p",
                    (NO, PermissionSet::ALL),
                    &[("t", NO, PermissionSet::ALL)],
                )],
                P::ProjectPlan,
                true,
                S::Fixed,
                None,
            ),
        ]
    }

    /// Every setting (allow, inherit, deny) of one permission: for the
    /// role and two teams on one level (27, both tiers, base on and off),
    /// and for the role and one team on each of two levels (81, a member,
    /// base on and off). The expectation is the resolver's; the web must
    /// agree with every one.
    pub fn grid() -> Vec<Case> {
        use WorkspaceMemberRole as R;
        let states = |p: Permission| {
            [
                (PermissionSet::EMPTY.with(p), NO),
                (NO, NO),
                (NO, PermissionSet::EMPTY.with(p)),
            ]
        };
        let mut out = Vec::new();
        let mut push = |name: String, tier, base, levels: Vec<OverrideLevel>, permission| {
            let entry = super::trace(tier, base, &levels, permission);
            out.push(Case {
                name,
                tier,
                base,
                levels,
                permission,
                allowed: entry.allowed,
                source: entry.source,
                team_id: entry.team_id,
            });
        };
        for (tier, permission) in [
            (R::Member, Permission::TicketEdit),
            (R::Guest, Permission::CommentCreate),
        ] {
            let s = states(permission);
            for base_on in [true, false] {
                let base = if base_on {
                    PermissionSet::EMPTY.with(permission)
                } else {
                    NO
                };
                for (i, role) in s.iter().enumerate() {
                    for (j, a) in s.iter().enumerate() {
                        for (k, b) in s.iter().enumerate() {
                            let levels = vec![level(
                                OverridePlace::Project,
                                "p",
                                *role,
                                &[("a", a.0, a.1), ("b", b.0, b.1)],
                            )];
                            push(
                                format!(
                                    "one level {} {} {i}{j}{k}",
                                    tier.as_str(),
                                    u8::from(base_on)
                                ),
                                tier,
                                base,
                                levels,
                                permission,
                            );
                        }
                    }
                }
            }
        }
        let permission = Permission::TicketEdit;
        let s = states(permission);
        for base_on in [true, false] {
            let base = if base_on {
                PermissionSet::EMPTY.with(permission)
            } else {
                NO
            };
            for (i, team_ws) in s.iter().enumerate() {
                for (j, role_p) in s.iter().enumerate() {
                    for (k, team_p) in s.iter().enumerate() {
                        for (l, team_c) in s.iter().enumerate() {
                            let levels = vec![
                                level(
                                    OverridePlace::Workspace,
                                    "ws",
                                    (NO, NO),
                                    &[("a", team_ws.0, team_ws.1)],
                                ),
                                level(
                                    OverridePlace::Project,
                                    "p",
                                    *role_p,
                                    &[("a", team_p.0, team_p.1)],
                                ),
                                level(
                                    OverridePlace::DocCollection,
                                    "c",
                                    (NO, NO),
                                    &[("a", team_c.0, team_c.1)],
                                ),
                            ];
                            push(
                                format!("three levels {} {i}{j}{k}{l}", u8::from(base_on)),
                                R::Member,
                                base,
                                levels,
                                permission,
                            );
                        }
                    }
                }
            }
        }
        out
    }

    /// The cases as JSON, one per line.
    pub fn to_json() -> Result<String, serde_json::Error> {
        let mut out = String::from("{\n  \"examples\": [\n");
        let lines = |cases: Vec<Case>| -> Result<String, serde_json::Error> {
            Ok(cases
                .iter()
                .map(|c| serde_json::to_string(c).map(|l| format!("    {l}")))
                .collect::<Result<Vec<_>, _>>()?
                .join(",\n"))
        };
        out.push_str(&lines(examples())?);
        out.push_str("\n  ],\n  \"grid\": [\n");
        out.push_str(&lines(grid())?);
        out.push_str("\n  ]\n}\n");
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{RolePreset, MEMBER_DEFAULT};

    #[test]
    fn the_overridable_sets_follow_the_concept() {
        use Permission as P;
        assert!(!OverridePlace::Project
            .overridable()
            .contains(P::ProjectCreate));
        assert!(!OverridePlace::Project.overridable().contains(P::DocCreate));
        assert!(OverridePlace::DocCollection
            .overridable()
            .contains(P::DocManage));
        assert!(!OverridePlace::DocCollection
            .overridable()
            .contains(P::TicketEdit));
        for p in [P::InviteCreate, P::InviteManage] {
            for place in OverridePlace::ALL {
                assert!(!p.overridable_at(place), "{p:?} at {place:?}");
            }
        }
        assert!(P::ProjectCreate.overridable_at(OverridePlace::Workspace));
        assert!(!OverridePlace::Workspace.takes(OverrideTarget::Role));
        assert!(OverridePlace::Workspace.takes(OverrideTarget::Team));
        assert!(OverridePlace::Project.takes(OverrideTarget::Role));
    }

    #[test]
    fn masks_refuse_both_and_read_both_as_deny() {
        let p = PermissionSet::EMPTY.with(Permission::TicketEdit);
        assert!(OverrideMasks::new(p, p).is_none());
        let stored = OverrideMasks::from_bits(p.bits(), p.bits());
        assert_eq!(stored.state(Permission::TicketEdit), OverrideState::Deny);
    }

    #[test]
    fn the_hot_path_and_the_trace_agree_on_every_case() {
        for case in cases::examples().into_iter().chain(cases::grid()) {
            let masks: Vec<LevelMasks> = case.levels.iter().map(OverrideLevel::masks).collect();
            let fast = resolve(case.tier, case.base, &masks).contains(case.permission);
            let traced = resolve_traced(case.tier, case.base, &case.levels)
                .into_iter()
                .find(|e| e.permission == case.permission)
                .expect("traced");
            assert_eq!(fast, case.allowed, "{}: fast", case.name);
            assert_eq!(traced.allowed, case.allowed, "{}: traced", case.name);
            assert_eq!(traced.source, case.source, "{}: source", case.name);
            assert_eq!(traced.team_id, case.team_id, "{}: team", case.name);
        }
    }

    #[test]
    fn without_overrides_the_role_is_all_there_is() {
        let dev = RolePreset::Developer.permissions();
        assert_eq!(resolve(WorkspaceMemberRole::Member, dev, &[]), dev);
        assert_eq!(
            resolve(WorkspaceMemberRole::Guest, MEMBER_DEFAULT, &[]),
            MEMBER_DEFAULT.intersection(GUEST_CEILING)
        );
        assert_eq!(
            resolve(WorkspaceMemberRole::Admin, PermissionSet::EMPTY, &[]),
            PermissionSet::ALL
        );
    }
}
