//! The policy as a hand-written truth table (`POLICY_PLAN.md` §6.3,
//! custom roles `CONCEPT.md` §3), the escalation cases (§7), and the
//! revision that pins the generated matrix.
//!
//! The table is written out, not derived: it is the decision (J1–J9,
//! E1–E8) in the form it was approved, and `allows()` has to meet it. The
//! matrix the web tests against (`src/generated/policy/policy-matrix.json`)
//! is derived from `allows()`; its fingerprint is recorded below with the
//! revision, so a changed verdict cannot ship under the old
//! `POLICY_REVISION`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use athene_contracts::auth::UserRole;
use athene_contracts::policy::{
    allows, assignable, capabilities, effective_permissions, grantable_roles, link_invite_allows,
    matrix, may_grant, requires, Action, CapabilitiesRole, Facts, Membership, Need, Permission,
    PermissionSet, PermissionSource, RolePreset, Standing, Subject, GUEST_CEILING, MEMBER_DEFAULT,
    POLICY_REVISION,
};
use athene_contracts::workspace::WorkspaceMemberRole as R;

/// Every revision of the rules with the fingerprint of its matrix
/// (`matrix::digest`). Changing a verdict: append `(POLICY_REVISION + 1,
/// "<new digest>")` and bump `POLICY_REVISION` in `src/policy.rs`.
///
/// 5: custom roles (phase A) — permissions per role, `roles.manage`,
/// `doc.create`, `doc.collection.manage`, `ticket.take`, `ticket.close`;
/// "Mitglied" no longer creates projects (E5); `ticket.plan` is a tick
/// (E8); the matrix gains the presets, a custom, an empty and a capped
/// guest role.
const RECORDED: &[(u32, &str)] = &[
    (1, "61b199d11696ad0c"),
    (2, "aaa9fb11da7f0950"),
    (3, "db757b6523ad1ec2"),
    (4, "18c7f2144c23bb8d"),
    (5, "d6c7fa24109644bd"),
];

/// Subjects, in the table's column order.
const COLUMNS: [&str; 16] = [
    "O", "A", "M", "G", "OpM", "AdM", "AdG", "OpG", "PgM", "Dev", "Tst", "PO", "Cus", "Nil", "GsW",
    "X",
];

fn subject(name: &str) -> Subject {
    matrix::subjects()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, s)| s)
        .unwrap_or_else(|| panic!("no subject {name}"))
}

/// A cell: `Y` allowed, `N` refused, `Own` only on the actor's own
/// resource, `Slf` only on the actor themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cell {
    Y,
    N,
    Own,
    Slf,
}
use Cell::{Own, Slf, N, Y};

/// The approved table: one row per action, columns as [`COLUMNS`]. Role
/// limits of `invite.create` / `members.*` are checked separately below;
/// here they run with no role named.
#[rustfmt::skip]
const TABLE: &[(Action, [Cell; 16])] = &[
    //                              O    A    M    G    OpM  AdM  AdG  OpG  PgM  Dev  Tst  PO   Cus  Nil  GsW  X
    (Action::WorkspaceRead,        [Y,   Y,   Y,   Y,   Y,   Y,   Y,   Y,   Y,   Y,   Y,   Y,   Y,   Y,   Y,   N]),
    (Action::WorkspaceSettingsEdit,[Y,   Y,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N]),
    // E4: roles stay with owners and admins, whatever a role says.
    (Action::RolesManage,          [Y,   Y,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N]),
    (Action::InviteManage,         [Y,   Y,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N,   Y,   N,   N,   N]),
    (Action::InviteCreate,         [Y,   Y,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N,   Y,   N,   N,   N]),
    (Action::MembersRoleChange,    [Y,   Y,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N,   N]),
    (Action::MembersRemove,        [Y,   Y,   Slf, Slf, Slf, Slf, Slf, Slf, N,   Slf, Slf, Slf, Slf, Slf, Slf, N]),
    (Action::TeamsManage,          [Y,   Y,   N,   N,   Y,   Y,   N,   N,   N,   N,   N,   Y,   N,   N,   N,   N]),
    (Action::LabelsManage,         [Y,   Y,   N,   N,   Y,   Y,   N,   N,   N,   N,   N,   Y,   N,   N,   N,   N]),
    (Action::TicketTypesManage,    [Y,   Y,   N,   N,   Y,   Y,   N,   N,   N,   N,   N,   Y,   N,   N,   N,   N]),
    (Action::FieldsManage,         [Y,   Y,   N,   N,   Y,   Y,   N,   N,   N,   N,   N,   Y,   N,   N,   N,   N]),
    (Action::ProjectPlan,          [Y,   Y,   N,   N,   Y,   Y,   N,   N,   N,   N,   N,   Y,   N,   N,   N,   N]),
    // E8: a tick; "Mitglied" has it unless the workspace said `managers`.
    (Action::TicketPlan,           [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   Y,   N,   Y,   N,   N,   N,   N]),
    (Action::CommentModerate,      [Y,   Y,   N,   N,   Y,   Y,   N,   N,   N,   N,   N,   Y,   N,   N,   N,   N]),
    (Action::TimeModerate,         [Y,   Y,   N,   N,   Y,   Y,   N,   N,   N,   N,   N,   Y,   N,   N,   N,   N]),
    (Action::TeamsMemberRemove,    [Y,   Y,   Slf, Slf, Y,   Y,   Slf, Slf, N,   Slf, Slf, Y,   Slf, Slf, Slf, N]),
    // E5: "Mitglied" no longer creates projects; Developer neither.
    (Action::ProjectCreate,        [Y,   Y,   N,   N,   Y,   Y,   N,   N,   N,   N,   N,   Y,   N,   N,   N,   N]),
    (Action::TicketCreate,         [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   Y,   Y,   Y,   N,   N,   N,   N]),
    (Action::TicketEdit,           [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   Y,   Y,   Y,   Y,   N,   N,   N]),
    (Action::TicketTake,           [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   Y,   Y,   Y,   N,   N,   N,   N]),
    (Action::TicketClose,          [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   Y,   Y,   Y,   N,   N,   N,   N]),
    (Action::TicketCheck,          [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   Y,   Y,   Y,   N,   N,   N,   N]),
    (Action::TimeLog,              [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   Y,   Y,   Y,   N,   N,   N,   N]),
    (Action::DocCreate,            [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   Y,   N,   Y,   N,   N,   N,   N]),
    (Action::DocWrite,             [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   Y,   N,   Y,   N,   N,   N,   N]),
    (Action::ProjectEdit,          [Y,   Y,   Own, N,   Y,   Y,   N,   N,   N,   Own, N,   Y,   N,   N,   N,   N]),
    (Action::TicketDelete,         [Y,   Y,   Own, N,   Y,   Y,   N,   N,   N,   Own, Own, Y,   N,   N,   N,   N]),
    (Action::TimeDelete,           [Y,   Y,   Own, N,   Y,   Y,   N,   N,   N,   Own, Own, Y,   N,   N,   N,   N]),
    (Action::DocDelete,            [Y,   Y,   Own, N,   Y,   Y,   N,   N,   N,   Own, N,   Y,   N,   N,   N,   N]),
    (Action::DocCollectionManage,  [Y,   Y,   Own, N,   Y,   Y,   N,   N,   N,   Own, N,   Y,   N,   N,   N,   N]),
    // UX9: events ("Termine"). Members create and edit, guests read; the
    // creator or a manager deletes.
    (Action::EventCreate,          [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   Y,   Y,   Y,   N,   N,   N,   N]),
    (Action::EventEdit,            [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   Y,   Y,   Y,   N,   N,   N,   N]),
    (Action::EventDelete,          [Y,   Y,   Own, N,   Y,   Y,   N,   N,   N,   Own, Own, Y,   N,   N,   N,   N]),
    (Action::TicketWatch,          [Y,   Y,   Y,   Y,   Y,   Y,   Y,   Y,   N,   Y,   Y,   Y,   Y,   N,   Y,   N]),
    (Action::CommentCreate,        [Y,   Y,   Y,   Y,   Y,   Y,   Y,   Y,   N,   Y,   Y,   Y,   Y,   N,   Y,   N]),
    (Action::CommentEdit,          [Own, Own, Own, Own, Own, Own, Own, Own, N,   Own, Own, Own, Own, Own, Own, N]),
    (Action::CommentDelete,        [Y,   Y,   Own, Own, Y,   Y,   Own, Own, N,   Own, Own, Y,   Own, Own, Own, N]),
    // J2: the admin console is the platform admin's, whatever the
    // membership; an operator is no admin.
    (Action::AdminAccess,          [N,   N,   N,   N,   N,   Y,   Y,   N,   N,   N,   N,   N,   N,   N,   N,   N]),
    (Action::WorkspaceCreate,      [Y,   Y,   Y,   Y,   Y,   Y,   Y,   Y,   N,   Y,   Y,   Y,   Y,   Y,   Y,   Y]),
    (Action::SlaRead,              [N,   N,   N,   N,   Y,   Y,   Y,   Y,   N,   N,   N,   N,   N,   N,   N,   N]),
    (Action::SlaManage,            [N,   N,   N,   N,   Y,   Y,   Y,   Y,   N,   N,   N,   N,   N,   N,   N,   N]),
];

fn verdicts(subject: &Subject, action: Action) -> Cell {
    let any = allows(subject, action, &Facts::NONE);
    let own = allows(subject, action, &Facts::owned(true));
    let me = allows(subject, action, &Facts::target(None, true));
    match (any, own, me) {
        (true, _, _) => Y,
        (false, true, _) => Own,
        (false, false, true) => Slf,
        (false, false, false) => N,
    }
}

#[test]
fn the_table_covers_every_action_once() {
    let mut listed: Vec<Action> = TABLE.iter().map(|(a, _)| *a).collect();
    listed.sort();
    let mut all = Action::ALL.to_vec();
    all.sort();
    assert_eq!(listed, all);
}

#[test]
fn every_cell_of_the_approved_table_holds() {
    let mut failures = Vec::new();
    for (action, row) in TABLE {
        for (column, expected) in COLUMNS.iter().zip(row) {
            let got = verdicts(&subject(column), *action);
            if got != *expected {
                failures.push(format!(
                    "{} as {column}: expected {expected:?}, got {got:?}",
                    action.as_str()
                ));
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// Every action's rule names a permission or is governance, membership or
/// platform: no action is decided by a tier comparison any more.
#[test]
fn every_member_action_is_a_permission_or_governance() {
    for action in Action::ALL {
        let need = requires(action);
        match need {
            Need::Platform => assert_eq!(
                action.scope(),
                athene_contracts::policy::ActionScope::Platform
            ),
            Need::Member | Need::Governance(_) => assert!(need.permissions().is_empty()),
            _ => assert!(
                !need.permissions().is_empty() || action == Action::CommentEdit,
                "{} reads no permission",
                action.as_str()
            ),
        }
    }
}

/// A role's verdict is exactly its ticks: one permission flipped changes
/// exactly the actions that read it.
#[test]
fn a_custom_role_gets_exactly_what_its_ticks_say() {
    for permission in Permission::ALL {
        let only = Subject::new(
            UserRole::User,
            Some(Membership::new(R::Member, PermissionSet::of(&[permission]))),
        );
        for action in Action::ALL {
            let need = requires(action);
            let reads = need.permissions().contains(&permission);
            let free = allows(
                &Subject::new(
                    UserRole::User,
                    Some(Membership::new(R::Member, PermissionSet::EMPTY)),
                ),
                action,
                &Facts::owned(true),
            );
            let with =
                allows(&only, action, &Facts::owned(true)) || allows(&only, action, &Facts::NONE);
            if !reads && !free {
                assert!(
                    !with,
                    "{} grants {} although its rule does not read it",
                    permission.as_str(),
                    action.as_str()
                );
            }
        }
    }
}

/// Operators and admins without a membership, and platform guests without
/// one: nothing inside a workspace; the platform actions by platform role.
#[test]
fn people_outside_the_workspace_keep_only_their_platform_rights() {
    for name in ["X", "OpX", "AdX", "PgX"] {
        let s = subject(name);
        for action in Action::ALL {
            let platform = matches!(
                action,
                Action::AdminAccess | Action::WorkspaceCreate | Action::SlaRead | Action::SlaManage
            );
            if !platform {
                assert_eq!(verdicts(&s, action), N, "{} as {name}", action.as_str());
            }
        }
    }
    assert!(allows(&subject("AdX"), Action::AdminAccess, &Facts::NONE));
    assert!(!allows(&subject("OpX"), Action::AdminAccess, &Facts::NONE));
    assert!(allows(&subject("OpX"), Action::SlaManage, &Facts::NONE));
    assert!(!allows(&subject("X"), Action::SlaRead, &Facts::NONE));
    assert!(!allows(
        &subject("PgX"),
        Action::WorkspaceCreate,
        &Facts::NONE
    ));
}

/// A project outside any workspace is the platform admin's alone, as a
/// manager — whatever their membership elsewhere; operators too are out.
#[test]
fn unplaced_data_is_the_platform_admins_alone() {
    let unplaced = Facts::unplaced();
    let owner_like = subject("O");
    for action in Action::ALL.into_iter().filter(|a| {
        matches!(
            a.scope(),
            athene_contracts::policy::ActionScope::Project
                | athene_contracts::policy::ActionScope::Ticket
        ) || *a == Action::ProjectCreate
    }) {
        for name in ["AdX", "AdM", "AdG"] {
            assert_eq!(
                allows(&subject(name), action, &unplaced),
                allows(&owner_like, action, &Facts::NONE),
                "{} as {name}",
                action.as_str()
            );
        }
        for name in ["O", "A", "M", "OpM", "OpG", "OpX", "X", "PgM", "PO", "Cus"] {
            assert!(
                !allows(&subject(name), action, &unplaced),
                "{} as {name} on unplaced data",
                action.as_str()
            );
        }
    }
}

#[test]
fn nobody_invites_above_their_own_role_and_links_stop_at_member() {
    let granted = |name: &str| -> Vec<R> {
        R::ALL
            .into_iter()
            .filter(|r| {
                allows(
                    &subject(name),
                    Action::InviteCreate,
                    &Facts::NONE.granting(*r),
                )
            })
            .collect()
    };
    assert_eq!(granted("O"), vec![R::Owner, R::Admin, R::Member, R::Guest]);
    assert_eq!(granted("A"), vec![R::Admin, R::Member, R::Guest]);
    // A tier named without its role's permissions hands out nothing to
    // someone who does not govern.
    for name in ["M", "G", "OpM", "AdM", "PgM", "Cus", "PO", "X"] {
        assert!(granted(name).is_empty(), "{name} invites");
    }
    assert_eq!(
        grantable_roles(&subject("O")),
        vec![R::Guest, R::Member, R::Admin, R::Owner]
    );
    assert_eq!(
        grantable_roles(&subject("A")),
        vec![R::Guest, R::Member, R::Admin]
    );
    assert_eq!(grantable_roles(&subject("Cus")), vec![R::Guest, R::Member]);
    assert!(grantable_roles(&subject("OpM")).is_empty());
    assert!(grantable_roles(&subject("PO")).is_empty());
    // J6
    assert!(link_invite_allows(R::Member));
    assert!(link_invite_allows(R::Guest));
    assert!(!link_invite_allows(R::Admin));
    assert!(!link_invite_allows(R::Owner));
}

// ---------------------------------------------------------------------------
// Escalation (§7): every path a person could use to gain rights.
// ---------------------------------------------------------------------------

/// A custom role that may invite hands out only roles whose ticks are a
/// subset of its own, of tier member or guest.
#[test]
fn escalation_inviting_into_a_higher_role_is_refused() {
    let cus = subject("Cus");
    let custom = matrix::CUSTOM;
    assert!(may_grant(&cus, R::Member, custom), "its own role");
    assert!(may_grant(
        &cus,
        R::Member,
        custom.without(Permission::InviteCreate)
    ));
    assert!(may_grant(&cus, R::Member, PermissionSet::EMPTY));
    assert!(may_grant(&cus, R::Guest, GUEST_CEILING));
    for (tier, grants, what) in [
        (
            R::Member,
            MEMBER_DEFAULT,
            "Mitglied (ticket.create it lacks)",
        ),
        (
            R::Member,
            RolePreset::ProductOwner.permissions(),
            "Product Owner",
        ),
        (
            R::Member,
            custom.with(Permission::LabelsManage),
            "one more tick",
        ),
        (R::Admin, PermissionSet::ALL, "admin"),
        (R::Owner, PermissionSet::ALL, "owner"),
        (
            R::Admin,
            PermissionSet::EMPTY,
            "an admin tier with no ticks",
        ),
    ] {
        assert!(!may_grant(&cus, tier, grants), "Cus invites into {what}");
    }
    // A preset that may invite: Developer + invite.create hands out
    // Tester (a subset), never Product Owner.
    let dev = Subject::new(
        UserRole::User,
        Some(Membership::new(
            R::Member,
            RolePreset::Developer
                .permissions()
                .with(Permission::InviteCreate),
        )),
    );
    assert!(may_grant(&dev, R::Member, RolePreset::Tester.permissions()));
    assert!(!may_grant(
        &dev,
        R::Member,
        RolePreset::ProductOwner.permissions()
    ));
    // Owners and admins hand out any role up to their tier.
    assert!(may_grant(&subject("A"), R::Admin, PermissionSet::ALL));
    assert!(!may_grant(&subject("A"), R::Owner, PermissionSet::ALL));
    assert!(may_grant(&subject("O"), R::Owner, PermissionSet::ALL));
    // Without the tick, nothing at all.
    assert!(!may_grant(&subject("PO"), R::Member, PermissionSet::EMPTY));
    assert!(!may_grant(&subject("M"), R::Guest, GUEST_CEILING));
}

/// Assigning a role is governance: no tick gives it, and admins never
/// touch owners or hand out the owner tier.
#[test]
fn escalation_assigning_a_higher_role_is_governance() {
    let may = |name: &str, target: R, to: R| {
        allows(
            &subject(name),
            Action::MembersRoleChange,
            &Facts::target(Some(target), false).granting(to),
        )
    };
    for target in R::ALL {
        for to in R::ALL {
            assert!(may("O", target, to), "owner: {target:?} -> {to:?}");
            let expected = target != R::Owner && to != R::Owner;
            assert_eq!(
                may("A", target, to),
                expected,
                "admin: {target:?} -> {to:?}"
            );
        }
    }
    for name in [
        "M", "G", "OpM", "AdM", "AdG", "PgM", "Dev", "Tst", "PO", "Cus", "Nil", "GsW", "X",
    ] {
        for to in R::ALL {
            assert!(!may(name, R::Guest, to), "{name} assigns {to:?}");
        }
    }
}

/// Editing a role — one's own included — and deleting the role one holds
/// are `roles.manage`, governance: no tick of any role grants it.
#[test]
fn escalation_editing_or_deleting_ones_own_role_is_governance() {
    let everything_but_governance = Subject::new(
        UserRole::User,
        Some(Membership::new(R::Member, PermissionSet::ALL)),
    );
    for s in [
        everything_but_governance.clone(),
        subject("Cus"),
        subject("PO"),
        subject("OpM"),
        subject("AdM"),
    ] {
        assert!(!allows(&s, Action::RolesManage, &Facts::NONE));
        assert!(!allows(&s, Action::WorkspaceSettingsEdit, &Facts::NONE));
        assert!(!allows(&s, Action::MembersRoleChange, &Facts::NONE));
        assert!(!allows(
            &s,
            Action::MembersRoleChange,
            &Facts::target(None, true).granting(R::Admin)
        ));
        assert!(!allows(
            &s,
            Action::MembersRemove,
            &Facts::target(Some(R::Member), false)
        ));
    }
    assert!(allows(&subject("A"), Action::RolesManage, &Facts::NONE));
    assert!(allows(&subject("O"), Action::RolesManage, &Facts::NONE));
}

/// A guest role holds only the ceiling, whatever its stored mask says, and
/// a platform guest writes nothing whatever role they hold.
#[test]
fn escalation_a_guest_never_gains_write_rights() {
    let capped = subject("GsW");
    assert_eq!(capped.effective(), GUEST_CEILING);
    for action in [
        Action::TicketEdit,
        Action::TicketCreate,
        Action::InviteCreate,
        Action::DocWrite,
        Action::ProjectCreate,
    ] {
        assert!(!allows(&capped, action, &Facts::NONE), "{action:?}");
        assert!(!allows(&capped, action, &Facts::owned(true)), "{action:?}");
    }
    let everything = Subject::new(
        UserRole::User,
        Some(Membership::new(R::Guest, PermissionSet::ALL)),
    );
    assert_eq!(everything.effective(), GUEST_CEILING);
    let platform_guest_po = Subject::new(
        UserRole::Guest,
        Some(Membership::new(
            R::Member,
            RolePreset::ProductOwner.permissions(),
        )),
    );
    for action in Action::ALL {
        if action.kind() == athene_contracts::policy::ActionKind::Write {
            assert!(
                !allows(&platform_guest_po, action, &Facts::owned(true)),
                "platform guest: {action:?}"
            );
        }
    }
    // The preview says why.
    let preview = effective_permissions(R::Guest, PermissionSet::ALL);
    let edit = preview
        .iter()
        .find(|e| e.permission == Permission::TicketEdit)
        .unwrap();
    assert!(!edit.allowed);
    assert_eq!(edit.source, PermissionSource::GuestCeiling);
    let fixed = effective_permissions(R::Admin, PermissionSet::EMPTY);
    assert!(fixed
        .iter()
        .all(|e| e.allowed && e.source == PermissionSource::Fixed));
}

#[test]
fn roles_change_downwards_from_the_top_and_owners_only_by_owners() {
    // J7: one's own role only downwards (the last owner is the store's).
    let own = |name: &str, to: R| {
        allows(
            &subject(name),
            Action::MembersRoleChange,
            &Facts::target(None, true).granting(to),
        )
    };
    assert!(own("O", R::Admin) && own("O", R::Member) && own("O", R::Owner));
    assert!(own("A", R::Member) && own("A", R::Guest) && own("A", R::Admin));
    assert!(!own("A", R::Owner));
    assert!(
        !own("M", R::Guest),
        "a member changes no role, not even their own"
    );
}

#[test]
fn removing_people_is_governing_and_leaving_is_everyones() {
    let may = |name: &str, target: R| {
        allows(
            &subject(name),
            Action::MembersRemove,
            &Facts::target(Some(target), false),
        )
    };
    for target in R::ALL {
        assert!(may("O", target));
        assert_eq!(may("A", target), target != R::Owner);
        assert!(!may("M", target));
        assert!(
            !may("OpM", target),
            "an operator member does not govern (J1)"
        );
        assert!(!may("Cus", target), "inviting is not removing (E4)");
    }
    for name in ["O", "A", "M", "G", "OpM", "AdM", "AdG", "OpG", "Nil", "GsW"] {
        assert!(allows(
            &subject(name),
            Action::MembersRemove,
            &Facts::target(None, true)
        ));
    }
    assert!(!allows(
        &subject("PgM"),
        Action::MembersRemove,
        &Facts::target(None, true)
    ));
}

/// J8: workspace guests may be assigned; platform guests and outsiders
/// may not.
#[test]
fn work_goes_to_members_including_guests_but_never_to_platform_guests() {
    assert!(assignable(&subject("M"), false));
    assert!(assignable(&subject("G"), false));
    assert!(assignable(&subject("AdG"), false));
    assert!(assignable(&subject("Nil"), false));
    assert!(!assignable(&subject("PgM"), false));
    assert!(!assignable(&subject("X"), false));
    assert!(assignable(&subject("X"), true));
    assert!(!assignable(&subject("PgX"), true));
}

/// E8: "Tickets einplanen" is the role's tick; sprints and releases
/// themselves stay with "Sprints und Releases verwalten".
#[test]
fn ticket_planning_is_the_roles_tick() {
    let without = Subject::new(
        UserRole::User,
        Some(Membership::new(
            R::Member,
            MEMBER_DEFAULT.without(Permission::TicketPlan),
        )),
    );
    assert!(!allows(&without, Action::TicketPlan, &Facts::NONE));
    assert!(allows(&without, Action::TicketEdit, &Facts::NONE));
    assert!(allows(&subject("M"), Action::TicketPlan, &Facts::NONE));
    assert!(!allows(&subject("M"), Action::ProjectPlan, &Facts::NONE));
    assert!(!Action::TicketPlan.contextual());
}

fn role() -> CapabilitiesRole {
    CapabilitiesRole {
        id: Some("r".into()),
        name: "Mitglied".into(),
        color: None,
        system_key: Some(R::Member),
    }
}

#[test]
fn capabilities_say_what_allows_says() {
    for (name, s) in matrix::subjects() {
        let caps = capabilities(&s, "ws".into(), role(), 7);
        let Some(caps) = caps else {
            assert!(
                s.membership.is_none(),
                "{name}: a member without capabilities"
            );
            continue;
        };
        assert_eq!(Some(caps.tier), s.tier());
        assert_eq!(caps.policy_revision, POLICY_REVISION);
        assert_eq!(caps.roles_epoch, 7);
        assert_eq!(caps.permissions, s.effective());
        assert_eq!(caps.actions.len(), Action::ALL.len());
        for action in Action::ALL {
            assert_eq!(
                caps.actions[&action],
                allows(&s, action, &Facts::NONE),
                "{name}"
            );
            if action.contextual() {
                let own = Facts {
                    owns: true,
                    target_is_self: true,
                    ..Facts::NONE
                };
                assert_eq!(caps.on_own[&action], allows(&s, action, &own), "{name}");
            } else {
                assert!(!caps.on_own.contains_key(&action), "{name}: {action:?}");
            }
        }
        assert_eq!(caps.grantable_roles, grantable_roles(&s));
    }
    let member = capabilities(&subject("M"), "ws".into(), role(), 1).unwrap();
    assert_eq!(member.standing, Standing::Contributor);
    assert!(!member.actions[&Action::TicketDelete]);
    assert!(member.on_own[&Action::TicketDelete]);
    assert!(!member.actions[&Action::ProjectCreate], "E5");
    let json = serde_json::to_value(&member).unwrap();
    assert_eq!(json["actions"]["ticket.delete"], false);
    assert_eq!(json["on_own"]["ticket.delete"], true);
    assert_eq!(json["standing"], "contributor");
    assert_eq!(json["tier"], "member");
    assert_eq!(json["role"]["name"], "Mitglied");
    assert!(json["permissions"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("ticket.edit")));
}

#[test]
fn the_matrix_holds_every_subject_action_and_variant() {
    let m = matrix::build();
    let per_subject: usize = Action::ALL.iter().map(|a| matrix::variants(*a).len()).sum();
    assert_eq!(m.cases.len(), per_subject * matrix::subjects().len());
    assert_eq!(m.actions.len(), Action::ALL.len());
    for case in &m.cases {
        assert_eq!(
            case.allowed,
            allows(&subject(case.subject), case.action, &case.facts)
        );
    }
    // The fixed roles, the presets and a custom role are all in it.
    for name in ["O", "A", "M", "G", "Dev", "Tst", "PO", "Cus"] {
        assert!(m.subjects.contains_key(name), "{name}");
    }
}

/// A changed verdict needs a new revision (G2): the capabilities answer
/// carries it, and a client holding rules of another revision reloads.
#[test]
fn the_policy_revision_matches_the_recorded_matrix() {
    let digest = matrix::digest(&matrix::build()).unwrap();
    let (revision, recorded) = RECORDED.last().copied().unwrap();
    for pair in RECORDED.windows(2) {
        assert!(pair[1].0 == pair[0].0 + 1, "revisions must count up by one");
        assert_ne!(pair[1].1, pair[0].1, "a revision without a changed verdict");
    }
    assert_eq!(
        (revision, recorded),
        (POLICY_REVISION, digest.as_str()),
        "the policy matrix changed (digest {digest}). If the change is intended, \
         set POLICY_REVISION to {next} in src/policy.rs and append ({next}, \"{digest}\") \
         to RECORDED here.",
        next = revision + 1
    );
}
