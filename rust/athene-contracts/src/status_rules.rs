//! Rules on a workflow status (W4-G): the approval step ("Freigabe",
//! four-eyes) and work-in-progress limits. Both are set per status of a
//! workspace (W4-F) and may be overridden per project; both are off until a
//! workspace sets them, so nothing changes for one that does not.
//!
//! **Approval** ([`StatusApproval`]) is about moving a ticket *out of* a
//! status:
//! - who may: anyone who may edit the ticket, or only the holders of the
//!   listed roles and the members of the listed teams. Owners and admins of
//!   the workspace always may; the move is recorded as theirs
//!   (`approval.bypass`).
//! - four-eyes ([`FourEyes`]): the mover may not be the ticket's assignee,
//!   or not the person who moved it into the status, or neither. Owners and
//!   admins pass here too (recorded).
//! - where to: `allowed_targets`, the status keys it may go to (empty: any).
//!   This is the workflow's shape and holds for everyone.
//!
//! A refused move is `409 ATH-RES-002` with `details.reason`
//! [`APPROVAL_NOT_ALLOWED_REASON`], [`APPROVAL_FOUR_EYES_REASON`] or
//! [`APPROVAL_TARGET_NOT_ALLOWED_REASON`]. A move that passes a rule
//! carries `approval` ([`ApprovalNote`]) in its `ticket_status_changed`
//! activity line.
//!
//! **WIP limits** ([`StatusWipLimit`]) are about moving a ticket *into* a
//! status: at most `limit` tickets there — per column (every ticket in the
//! status at the rule's place: the project for a project's rule, the whole
//! workspace for the workspace's) or per person (each assignee's tickets
//! there). In mode `warn` the write lands and the answer carries
//! `warnings` ([`TicketWarning`], shown in orange); in mode `block` it is
//! `409 ATH-RES-002` with `details.reason` [`WIP_LIMIT_REACHED_REASON`],
//! unless an owner or admin sends `wip_override` with a reason, which the
//! activity line keeps (`wip_override`, [`WipOverrideNote`]).
//!
//! Every status write path checks both: `PATCH /work/tickets/:id/status`,
//! the board move (`…/move`) and the bulk edit (per ticket: a refused one
//! lands in `failed`). Taking from a queue does not change the status, the
//! hand-back and a status deleted with `move_to` are moves of the system:
//! none of them is checked.
//!
//! Routes (every member reads; owners and admins set the rules:
//! `workspace.settings.edit`, a project's too):
//! - `GET /work/workspaces/:workspace_id/statuses/rules`
//! - `PUT /work/workspaces/:workspace_id/statuses/:status_id/rules` — one
//!   status at one place, with `version` (3.7: `409 ATH-RES-003` with
//!   `details.current` when it moved).
//! - `GET /work/workspaces/:workspace_id/wip[?project_id=]` — the counts
//!   beside the limits, for a board's "3/5", in one aggregated read.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::Patch;

/// `409` (`details.reason`): the caller is none of the movers.
pub const APPROVAL_NOT_ALLOWED_REASON: &str = "approval_not_allowed";
/// `409`: four-eyes — the caller is the assignee, or moved it in.
pub const APPROVAL_FOUR_EYES_REASON: &str = "approval_four_eyes";
/// `409`: the status may not be left for that one.
pub const APPROVAL_TARGET_NOT_ALLOWED_REASON: &str = "approval_target_not_allowed";
/// `409`: a blocking WIP limit is reached.
pub const WIP_LIMIT_REACHED_REASON: &str = "wip_limit_reached";

/// The highest WIP limit.
pub const WIP_LIMIT_MAX: u32 = 999;
/// The longest reason of a WIP override, in characters.
pub const WIP_OVERRIDE_REASON_MAX_CHARS: usize = 500;
/// The most roles and teams an approval names (together).
pub const APPROVAL_MAX_MOVERS: usize = 50;

/// Where a rule is set.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "workflow/")]
pub enum RulePlace {
    /// Every project of the workspace (unless one says otherwise).
    Workspace,
    /// One project: replaces the workspace's rule there.
    Project,
}

/// Who may move a ticket out of a status with an approval.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "workflow/")]
pub enum ApprovalMovers {
    /// Whoever may edit the ticket.
    #[default]
    Anyone,
    /// The holders of `role_ids` and the members of `team_ids`.
    Listed,
}

/// The four-eyes rule of an approval.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "workflow/")]
pub enum FourEyes {
    #[default]
    Off,
    /// Not the ticket's assignee.
    NotAssignee,
    /// Not the person who moved it into the status (a ticket nobody moved
    /// since W4-G holds nobody back).
    NotEnteredBy,
    /// Neither of the two.
    Neither,
}

impl FourEyes {
    /// Whether the rule asks about the assignee.
    pub const fn checks_assignee(self) -> bool {
        matches!(self, Self::NotAssignee | Self::Neither)
    }

    /// Whether the rule asks about who moved the ticket in.
    pub const fn checks_entered_by(self) -> bool {
        matches!(self, Self::NotEnteredBy | Self::Neither)
    }
}

/// An approval step on a status: who may move a ticket out, under which
/// four-eyes rule, and where to. With `enabled: false` a project's rule
/// switches the workspace's off there (a workspace's rule that is off is
/// the same as none).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workflow/")]
pub struct StatusApproval {
    #[serde(default = "yes")]
    #[ts(optional, as = "Option<bool>")]
    pub enabled: bool,
    #[serde(default)]
    #[ts(optional, as = "Option<ApprovalMovers>")]
    pub movers: ApprovalMovers,
    /// With `listed`: roles of the workspace whose holders may move.
    #[serde(default)]
    #[ts(optional, as = "Option<Vec<String>>")]
    pub role_ids: Vec<String>,
    /// With `listed`: teams of the workspace whose members may move.
    #[serde(default)]
    #[ts(optional, as = "Option<Vec<String>>")]
    pub team_ids: Vec<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<FourEyes>")]
    pub four_eyes: FourEyes,
    /// Status keys a ticket may go to from here; empty: any.
    #[serde(default)]
    #[ts(optional, as = "Option<Vec<String>>")]
    pub allowed_targets: Vec<String>,
}

fn yes() -> bool {
    true
}

/// What a WIP limit counts.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "workflow/")]
pub enum WipScope {
    /// Every ticket in the status at the rule's place (the project for a
    /// project's rule, the whole workspace for the workspace's).
    #[default]
    Column,
    /// Each assignee's tickets in the status at the rule's place.
    Person,
}

/// What a WIP limit does when it is reached.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "workflow/")]
pub enum WipMode {
    /// No limit (a project's rule switches the workspace's off there).
    Off,
    /// The write lands; the answer warns.
    #[default]
    Warn,
    /// The write is refused, unless an owner or admin overrides it with a
    /// reason.
    Block,
}

/// A WIP limit on a status.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workflow/")]
pub struct StatusWipLimit {
    #[serde(default)]
    #[ts(optional, as = "Option<WipMode>")]
    pub mode: WipMode,
    /// 1–[`WIP_LIMIT_MAX`]; required unless `mode` is `off`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub limit: Option<u32>,
    #[serde(default)]
    #[ts(optional, as = "Option<WipScope>")]
    pub scope: WipScope,
}

/// The rules one status has at one place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "workflow/")]
pub struct StatusRuleResponse {
    pub id: String,
    pub status_id: String,
    pub status_key: String,
    pub place: RulePlace,
    /// The project, for a project's rule.
    pub project_id: Option<String>,
    /// `null`: no approval set here (a project's rule then follows the
    /// workspace's).
    pub approval: Option<StatusApproval>,
    /// `null`: no WIP limit set here.
    pub wip_limit: Option<StatusWipLimit>,
    /// For `PUT` (3.7).
    pub version: i64,
    pub updated_at: String,
    pub updated_by: Option<String>,
}

/// `GET /work/workspaces/:workspace_id/statuses/rules`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "workflow/")]
pub struct WorkflowRulesResponse {
    /// The workspace's rules first, then the projects', by status order.
    pub rules: Vec<StatusRuleResponse>,
    /// The caller may set them (`workspace.settings.edit`).
    pub editable: bool,
}

/// `PUT /work/workspaces/:workspace_id/statuses/:status_id/rules`: the
/// rules of one status at one place. Absent keeps a rule as it is, `null`
/// clears it (a project then follows the workspace again); with both
/// cleared the place has no rules left.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workflow/")]
pub struct SetStatusRulesRequest {
    /// A live project of the workspace; absent: the workspace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<StatusApproval>)]
    pub approval: Patch<StatusApproval>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<StatusWipLimit>)]
    pub wip_limit: Patch<StatusWipLimit>,
    /// The version the edit started from; `0` when the place had none.
    pub version: i64,
}

/// `PUT …/statuses/:status_id/rules`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "workflow/")]
pub struct SetStatusRulesResponse {
    /// `null`: nothing is set at the place any more.
    pub rule: Option<StatusRuleResponse>,
}

/// `wip_override` of a status write: an owner's or admin's reason to move
/// past a blocking limit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "workflow/")]
pub struct WipOverride {
    /// 1–[`WIP_OVERRIDE_REASON_MAX_CHARS`] characters, trimmed.
    pub reason: String,
}

/// What a write's answer warns about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "workflow/")]
pub enum TicketWarningKind {
    /// A WIP limit is reached or passed (mode `warn`, or an override).
    WipLimit,
}

/// A warning of a status write (the web shows it in orange).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "workflow/")]
pub struct TicketWarning {
    pub kind: TicketWarningKind,
    /// The ticket (a bulk edit's answer names it).
    pub ticket_id: String,
    /// The status key the limit is on.
    pub status: String,
    pub limit: u32,
    /// Tickets counted with this one in.
    pub current: i64,
    pub scope: WipScope,
    pub place: RulePlace,
    /// The person counted (scope `person`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub assignee_id: Option<String>,
}

/// The answer of a single write that a WIP limit can warn about: a status
/// write (`PATCH …/status`, `…/move`) and, per person, an assignment
/// (`PATCH …/assign`) and a take (`POST …/take`, W4-K): the ticket, and
/// `warnings` when a WIP limit warns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "workflow/")]
pub struct TicketWriteResponse {
    #[schema(value_type = Object)]
    pub data: crate::work::TicketResponse,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<TicketWarning>>")]
    pub warnings: Vec<TicketWarning>,
}

/// `approval` in the metadata of a `ticket_status_changed` line: a move
/// out of a status with an approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "workflow/")]
pub struct ApprovalNote {
    /// Who moved (approved) it.
    pub by: String,
    pub from: String,
    pub to: String,
    pub place: RulePlace,
    pub four_eyes: FourEyes,
    /// An owner or admin moved it past a rule that would have held anyone
    /// else back.
    pub bypass: bool,
}

/// `wip_override` in the metadata of a `ticket_status_changed` line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "workflow/")]
pub struct WipOverrideNote {
    pub reason: String,
    pub status: String,
    pub limit: u32,
    /// Tickets counted with this one in.
    pub current: i64,
    pub scope: WipScope,
    pub place: RulePlace,
}

/// `GET /work/workspaces/:workspace_id/wip?project_id=`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "workflow/", optional_fields)]
pub struct WipCountsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

/// One person's count under a `person` limit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "workflow/")]
pub struct WipPersonCount {
    pub assignee_id: String,
    pub count: i64,
}

/// A status with a WIP limit in effect, and its count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "workflow/")]
pub struct StatusWipCount {
    pub status: String,
    pub place: RulePlace,
    pub limit: u32,
    pub mode: WipMode,
    pub scope: WipScope,
    /// `column`: the tickets in the status at the place; `person`: the
    /// caller's there.
    pub current: i64,
    /// `person`: everyone with tickets there, most first.
    #[serde(default)]
    pub people: Vec<WipPersonCount>,
}

/// `GET …/wip`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "workflow/")]
pub struct WipCountsResponse {
    pub workspace_id: String,
    pub project_id: Option<String>,
    /// The statuses with a limit in effect here, in workflow order.
    pub counts: Vec<StatusWipCount>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_approval_needs_only_what_it_says() {
        let on: StatusApproval = serde_json::from_value(json!({})).expect("defaults");
        assert!(on.enabled);
        assert_eq!(on.movers, ApprovalMovers::Anyone);
        assert_eq!(on.four_eyes, FourEyes::Off);
        let listed: StatusApproval = serde_json::from_value(json!({
            "movers": "listed", "role_ids": ["r"], "four_eyes": "not_entered_by",
            "allowed_targets": ["done"]
        }))
        .expect("listed");
        assert!(listed.four_eyes.checks_entered_by() && !listed.four_eyes.checks_assignee());
        assert!(FourEyes::Neither.checks_assignee() && FourEyes::Neither.checks_entered_by());
        assert!(serde_json::from_value::<StatusApproval>(json!({ "who": "me" })).is_err());
    }

    #[test]
    fn a_rule_edit_tells_absent_from_null() {
        let req: SetStatusRulesRequest = serde_json::from_value(json!({
            "approval": null, "wip_limit": { "mode": "block", "limit": 3, "scope": "person" },
            "version": 2
        }))
        .expect("rules");
        assert!(req.approval.is_null());
        assert_eq!(
            req.wip_limit.value().map(|w| (w.mode, w.limit, w.scope)),
            Some((WipMode::Block, Some(3), WipScope::Person))
        );
        let none: SetStatusRulesRequest =
            serde_json::from_value(json!({ "version": 0 })).expect("nothing");
        assert!(none.approval.is_absent() && none.wip_limit.is_absent());
        assert!(
            serde_json::from_value::<SetStatusRulesRequest>(json!({ "version": 0, "x": 1 }))
                .is_err()
        );
        assert!(
            serde_json::from_value::<WipOverride>(json!({ "reason": "r", "force": true })).is_err()
        );
    }

    #[test]
    fn a_warning_names_the_person_only_when_counted() {
        let warning = TicketWarning {
            kind: TicketWarningKind::WipLimit,
            ticket_id: "t".into(),
            status: "qa".into(),
            limit: 3,
            current: 4,
            scope: WipScope::Column,
            place: RulePlace::Project,
            assignee_id: None,
        };
        let wire = serde_json::to_value(&warning).expect("warning");
        assert_eq!(wire["kind"], "wip_limit");
        assert!(wire.get("assignee_id").is_none());
    }
}
