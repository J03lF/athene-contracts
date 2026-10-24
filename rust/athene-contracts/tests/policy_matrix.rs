//! The policy as a hand-written truth table (`POLICY_PLAN.md` §6.3), and
//! the revision that pins the generated matrix.
//!
//! The table is written out, not derived: it is the decision (J1–J9) in
//! the form it was approved, and `allows()` has to meet it. The matrix the
//! web tests against (`src/generated/policy/policy-matrix.json`) is derived
//! from `allows()`; its fingerprint is recorded below with the revision, so
//! a changed verdict cannot ship under the old `POLICY_REVISION`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use athene_contracts::policy::{
    allows, assignable, capabilities, grantable_roles, link_invite_allows, matrix, Action, Facts,
    Standing, Subject, POLICY_REVISION,
};
use athene_contracts::workspace::WorkspaceMemberRole as R;

/// Every revision of the rules with the fingerprint of its matrix
/// (`matrix::digest`). Changing a verdict: append `(POLICY_REVISION + 1,
/// "<new digest>")` and bump `POLICY_REVISION` in `src/policy.rs`.
const RECORDED: &[(u32, &str)] = &[
    (1, "61b199d11696ad0c"),
    (2, "aaa9fb11da7f0950"),
    (3, "db757b6523ad1ec2"),
];

/// Subjects, in the table's column order.
const COLUMNS: [&str; 10] = ["O", "A", "M", "G", "OpM", "AdM", "AdG", "OpG", "PgM", "X"];

fn subject(name: &str) -> Subject {
    let (_, platform, membership) = matrix::SUBJECTS
        .iter()
        .find(|(n, _, _)| *n == name)
        .unwrap_or_else(|| panic!("no subject {name}"));
    Subject::new(platform.clone(), *membership)
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
const TABLE: &[(Action, [Cell; 10])] = &[
    //                              O    A    M    G    OpM  AdM  AdG  OpG  PgM  X
    (Action::WorkspaceRead,        [Y,   Y,   Y,   Y,   Y,   Y,   Y,   Y,   Y,   N]),
    (Action::WorkspaceSettingsEdit,[Y,   Y,   N,   N,   N,   N,   N,   N,   N,   N]),
    (Action::InviteManage,         [Y,   Y,   N,   N,   N,   N,   N,   N,   N,   N]),
    (Action::InviteCreate,         [Y,   Y,   N,   N,   N,   N,   N,   N,   N,   N]),
    (Action::MembersRoleChange,    [Y,   Y,   N,   N,   N,   N,   N,   N,   N,   N]),
    (Action::MembersRemove,        [Y,   Y,   Slf, Slf, Slf, Slf, Slf, Slf, N,   N]),
    (Action::TeamsManage,          [Y,   Y,   N,   N,   Y,   Y,   N,   N,   N,   N]),
    (Action::LabelsManage,         [Y,   Y,   N,   N,   Y,   Y,   N,   N,   N,   N]),
    (Action::TicketTypesManage,    [Y,   Y,   N,   N,   Y,   Y,   N,   N,   N,   N]),
    (Action::FieldsManage,         [Y,   Y,   N,   N,   Y,   Y,   N,   N,   N,   N]),
    (Action::ProjectPlan,          [Y,   Y,   N,   N,   Y,   Y,   N,   N,   N,   N]),
    // `ticket_planning = managers`; with `editors` see below.
    (Action::TicketPlan,           [Y,   Y,   N,   N,   Y,   Y,   N,   N,   N,   N]),
    (Action::CommentModerate,      [Y,   Y,   N,   N,   Y,   Y,   N,   N,   N,   N]),
    (Action::TimeModerate,         [Y,   Y,   N,   N,   Y,   Y,   N,   N,   N,   N]),
    (Action::TeamsMemberRemove,    [Y,   Y,   Slf, Slf, Y,   Y,   Slf, Slf, N,   N]),
    (Action::ProjectCreate,        [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   N]),
    (Action::TicketCreate,         [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   N]),
    (Action::TicketEdit,           [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   N]),
    (Action::TicketCheck,          [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   N]),
    (Action::TimeLog,              [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   N]),
    (Action::DocWrite,             [Y,   Y,   Y,   N,   Y,   Y,   N,   N,   N,   N]),
    (Action::ProjectEdit,          [Y,   Y,   Own, N,   Y,   Y,   N,   N,   N,   N]),
    (Action::TicketDelete,         [Y,   Y,   Own, N,   Y,   Y,   N,   N,   N,   N]),
    (Action::TimeDelete,           [Y,   Y,   Own, N,   Y,   Y,   N,   N,   N,   N]),
    (Action::DocDelete,            [Y,   Y,   Own, N,   Y,   Y,   N,   N,   N,   N]),
    (Action::TicketWatch,          [Y,   Y,   Y,   Y,   Y,   Y,   Y,   Y,   N,   N]),
    (Action::CommentCreate,        [Y,   Y,   Y,   Y,   Y,   Y,   Y,   Y,   N,   N]),
    (Action::CommentEdit,          [Own, Own, Own, Own, Own, Own, Own, Own, N,   N]),
    (Action::CommentDelete,        [Y,   Y,   Own, Own, Y,   Y,   Own, Own, N,   N]),
    // J2: the admin console is the platform admin's, whatever the
    // membership; an operator is no admin.
    (Action::AdminAccess,          [N,   N,   N,   N,   N,   Y,   Y,   N,   N,   N]),
    (Action::WorkspaceCreate,      [Y,   Y,   Y,   Y,   Y,   Y,   Y,   Y,   N,   Y]),
    (Action::SlaRead,              [N,   N,   N,   N,   Y,   Y,   Y,   Y,   N,   N]),
    (Action::SlaManage,            [N,   N,   N,   N,   Y,   Y,   Y,   Y,   N,   N]),
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
        for name in ["O", "A", "M", "OpM", "OpG", "OpX", "X", "PgM"] {
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
    for name in ["M", "G", "OpM", "AdM", "PgM", "X"] {
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
    assert!(grantable_roles(&subject("OpM")).is_empty());
    // J6
    assert!(link_invite_allows(R::Member));
    assert!(link_invite_allows(R::Guest));
    assert!(!link_invite_allows(R::Admin));
    assert!(!link_invite_allows(R::Owner));
}

#[test]
fn roles_change_downwards_from_the_top_and_owners_only_by_owners() {
    let may = |name: &str, target: R, to: R| {
        allows(
            &subject(name),
            Action::MembersRoleChange,
            &Facts::target(Some(target), false).granting(to),
        )
    };
    // An owner changes anyone to anything.
    for target in R::ALL {
        for to in R::ALL {
            assert!(may("O", target, to), "owner: {target:?} -> {to:?}");
        }
    }
    // An admin: not an owner, not to owner.
    for target in R::ALL {
        for to in R::ALL {
            let expected = target != R::Owner && to != R::Owner;
            assert_eq!(
                may("A", target, to),
                expected,
                "admin: {target:?} -> {to:?}"
            );
        }
    }
    // Nobody else.
    for name in ["M", "G", "OpM", "AdM", "AdG", "PgM", "X"] {
        assert!(!may(name, R::Guest, R::Member), "{name}");
    }
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
    }
    for name in ["O", "A", "M", "G", "OpM", "AdM", "AdG", "OpG"] {
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
    assert!(!assignable(&subject("PgM"), false));
    assert!(!assignable(&subject("X"), false));
    assert!(assignable(&subject("X"), true));
    assert!(!assignable(&subject("PgX"), true));
}

/// "Wer darf Tickets einplanen" (Joel, after R1): with `editors` (the
/// default) everyone who edits a ticket puts it into a sprint or a release;
/// with `managers` only managers. Sprints and releases themselves stay the
/// managers' either way.
#[test]
fn editors_plan_tickets_where_the_workspace_says_so() {
    let open = Facts::NONE.planning(true);
    for name in COLUMNS {
        let s = subject(name);
        assert_eq!(
            allows(&s, Action::TicketPlan, &open),
            allows(&s, Action::TicketEdit, &Facts::NONE),
            "ticket.plan with editors as {name}"
        );
        assert_eq!(
            allows(&s, Action::TicketPlan, &Facts::NONE),
            allows(&s, Action::ProjectPlan, &Facts::NONE),
            "ticket.plan with managers as {name}"
        );
        assert_eq!(
            allows(&s, Action::ProjectPlan, &open),
            allows(&s, Action::ProjectPlan, &Facts::NONE),
            "project.plan ignores the setting as {name}"
        );
    }
    assert!(!Action::TicketPlan.contextual());
    let member = capabilities(&subject("M"), "ws".into(), true).unwrap();
    assert!(member.actions[&Action::TicketPlan]);
    assert!(!member.actions[&Action::ProjectPlan]);
    assert!(!member.on_own.contains_key(&Action::TicketPlan));
    let member = capabilities(&subject("M"), "ws".into(), false).unwrap();
    assert!(!member.actions[&Action::TicketPlan]);
    let guest = capabilities(&subject("G"), "ws".into(), true).unwrap();
    assert!(!guest.actions[&Action::TicketPlan]);
}

#[test]
fn capabilities_say_what_allows_says() {
    for (name, _, membership) in matrix::SUBJECTS {
        let s = subject(name);
        let caps = capabilities(&s, "ws".into(), false);
        let Some(caps) = caps else {
            assert!(
                membership.is_none(),
                "{name}: a member without capabilities"
            );
            continue;
        };
        assert_eq!(Some(caps.role), membership);
        assert_eq!(caps.policy_revision, POLICY_REVISION);
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
    let member = capabilities(&subject("M"), "ws".into(), false).unwrap();
    assert_eq!(member.standing, Standing::Contributor);
    assert!(!member.actions[&Action::TicketDelete]);
    assert!(member.on_own[&Action::TicketDelete]);
    let json = serde_json::to_value(&member).unwrap();
    assert_eq!(json["actions"]["ticket.delete"], false);
    assert_eq!(json["on_own"]["ticket.delete"], true);
    assert_eq!(json["standing"], "contributor");
}

#[test]
fn the_matrix_holds_every_subject_action_and_variant() {
    let m = matrix::build();
    let per_subject: usize = Action::ALL.iter().map(|a| matrix::variants(*a).len()).sum();
    assert_eq!(m.cases.len(), per_subject * matrix::SUBJECTS.len());
    assert_eq!(m.actions.len(), Action::ALL.len());
    for case in &m.cases {
        assert_eq!(
            case.allowed,
            allows(&subject(case.subject), case.action, &case.facts)
        );
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
