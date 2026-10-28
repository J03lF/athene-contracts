//! Where a ticket field applies: rules, fields of one ticket, suggestions,
//! and the one resolution every side computes (custom fields wave 3, CF1;
//! `artifacts/custom-fields/CONCEPT.md` §2.2–§2.4, §4.2).
//!
//! - A **rule** (`athenos_field_assignments`) says "field F applies to the
//!   tickets that meet every condition it sets": a team, a ticket type, a
//!   project, any combination (E2). No condition = the whole workspace.
//!   Several rules of one field are alternatives. A rule may make the field
//!   required, at create (the default) or at close (E4), and may name a
//!   default of its own.
//! - A **field of one ticket** ("+ Feld", ad hoc) is an existing definition
//!   hung onto exactly that ticket; its value lives in `custom_values` like
//!   every other.
//! - A **suggestion** asks a manager to give a field (or a drafted one) a
//!   rule for a team, a type or a project (E3: only `fields.manage` creates
//!   a field from a ticket; everyone else proposes).
//!
//! [`resolve_fields`] is the resolution: a pure function, ported to
//! TypeScript (`src/payloads/field-assignments.ts`), and both are held to
//! the same cases ([`cases`], `src/generated/field_definitions/
//! field-resolution.json`), like the policy matrix. The core resolves on
//! every write and trusts no client.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::field_definitions::FieldType;
use crate::Patch;

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

/// When a required field must hold a value (E4).
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "field_definitions/")]
pub enum RequiredOn {
    /// When the ticket is created (and it cannot be emptied after).
    #[default]
    Create,
    /// When the ticket moves to done or closed; open tickets may leave it
    /// empty (a fix version, a test result).
    Done,
}

impl RequiredOn {
    pub const fn as_str(self) -> &'static str {
        match self {
            RequiredOn::Create => "create",
            RequiredOn::Done => "done",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "create" => Some(RequiredOn::Create),
            "done" => Some(RequiredOn::Done),
            _ => None,
        }
    }
}

/// One rule, as `GET /work/workspaces/:id/field-assignments` lists it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "field_definitions/")]
pub struct FieldAssignmentResponse {
    pub id: String,
    pub workspace_id: String,
    /// The definition (a ticket field of the same workspace).
    pub field_id: String,
    /// Condition: the ticket's team. `null` = any.
    pub team_id: Option<String>,
    /// Condition: the ticket's type key. `null` = any.
    pub ticket_type_key: Option<String>,
    /// Condition: the ticket's project. `null` = any.
    pub project_id: Option<String>,
    pub required: bool,
    /// Meaningful when `required`.
    pub required_on: RequiredOn,
    /// Overrides the definition's default where this rule is the most
    /// specific one with a default (§2.2 rule 3).
    #[ts(type = "unknown")]
    pub default_value: Option<JsonValue>,
    pub created_by: Option<String>,
    pub created_at: String,
    /// Rises with every write (3.7).
    pub version: i64,
}

/// A rule without its field: what `POST …/field-definitions` takes as
/// `rules`, and the body of `POST …/field-assignments` minus `field_id`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "field_definitions/", optional_fields)]
pub struct FieldRuleDraft {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket_type_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_on: Option<RequiredOn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub default_value: Option<JsonValue>,
}

/// `POST /work/workspaces/:id/field-assignments`.
///
/// `fields.manage` for every rule; a rule for one project also with
/// `project.edit` at that project, for an existing field (E6). The same
/// conditions twice are a `409`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "field_definitions/")]
pub struct CreateFieldAssignmentRequest {
    pub field_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub team_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ticket_type_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub required: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub required_on: Option<RequiredOn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "unknown")]
    pub default_value: Option<JsonValue>,
}

/// `PATCH /work/field-assignments/:id`: whether it requires, when, and its
/// default. The conditions are the rule; another rule is another row.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "field_definitions/")]
pub struct UpdateFieldAssignmentRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub required: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub required_on: Option<RequiredOn>,
    /// `null` drops the rule's own default.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<Object>)]
    #[ts(type = "unknown")]
    pub default_value: Patch<JsonValue>,
    /// Per changed part, the value the client saw: a moved one is a `409`
    /// `ATH-RES-003` with the rule as it is now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    #[schema(value_type = Option<Object>)]
    pub expected: Option<FieldAssignmentExpected>,
}

impl UpdateFieldAssignmentRequest {
    pub fn is_empty(&self) -> bool {
        self.required.is_none() && self.required_on.is_none() && self.default_value.is_absent()
    }
}

/// The base of a rule update.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "field_definitions/")]
pub struct FieldAssignmentExpected {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub required: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub required_on: Option<RequiredOn>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[ts(type = "unknown")]
    pub default_value: Patch<JsonValue>,
}

pub type FieldAssignmentListResponse = crate::ListResponse<FieldAssignmentResponse>;

// ---------------------------------------------------------------------------
// Fields of one ticket
// ---------------------------------------------------------------------------

/// `POST /work/tickets/:id/fields` (`field.attach`): hang an existing,
/// active ticket field onto this ticket. Attaching one it has is no change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "field_definitions/")]
pub struct AttachTicketFieldRequest {
    pub field_id: String,
}

/// `DELETE /work/tickets/:id/fields/:field_id` (`field.attach`). A field
/// that holds a value is detached only with `clear=true`, which empties
/// the value in the same write; otherwise `409`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "field_definitions/", optional_fields)]
pub struct DetachTicketFieldQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clear: Option<bool>,
}

// ---------------------------------------------------------------------------
// Suggestions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "field_definitions/")]
pub enum FieldSuggestionStatus {
    Open,
    Accepted,
    Declined,
}

impl FieldSuggestionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            FieldSuggestionStatus::Open => "open",
            FieldSuggestionStatus::Accepted => "accepted",
            FieldSuggestionStatus::Declined => "declined",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "open" => Some(FieldSuggestionStatus::Open),
            "accepted" => Some(FieldSuggestionStatus::Accepted),
            "declined" => Some(FieldSuggestionStatus::Declined),
            _ => None,
        }
    }
}

/// A field that does not exist yet, as somebody without `fields.manage`
/// proposes it from a ticket (E3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "field_definitions/")]
pub struct FieldDraft {
    /// 1..80 characters.
    pub label: String,
    /// A type without options of its own: text, long text, number, date,
    /// date and time, checkbox, URL, person. A choice or version field
    /// needs its choices, which the manager adds.
    pub field_type: FieldType,
}

impl FieldDraft {
    /// The types a draft may have.
    pub const TYPES: [FieldType; 8] = [
        FieldType::Text,
        FieldType::LongText,
        FieldType::Number,
        FieldType::Date,
        FieldType::Datetime,
        FieldType::Boolean,
        FieldType::Url,
        FieldType::User,
    ];
}

/// One open (or decided) suggestion. Open suggestions of the same field
/// for the same target are one entry, counted over their tickets
/// ("aus 3 Tickets").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "field_definitions/")]
pub struct FieldSuggestionResponse {
    /// The oldest suggestion of the entry; deciding it decides the entry.
    pub id: String,
    pub workspace_id: String,
    /// An existing field…
    pub field_id: Option<String>,
    /// …or a drafted one.
    pub draft: Option<FieldDraft>,
    /// Exactly one target.
    pub team_id: Option<String>,
    pub ticket_type_key: Option<String>,
    pub project_id: Option<String>,
    /// The tickets it was suggested from, oldest first.
    pub ticket_ids: Vec<String>,
    /// How many suggestions the entry holds.
    pub count: usize,
    /// Who suggested first.
    pub suggested_by: String,
    pub status: FieldSuggestionStatus,
    pub decided_by: Option<String>,
    pub decided_at: Option<String>,
    pub created_at: String,
}

/// `POST /work/workspaces/:id/field-suggestions` (`field.suggest` on the
/// ticket): propose `field_id` or a `draft` for one target of that ticket:
/// its team, its type or its project. With a draft, `value` is the value
/// wished for this ticket, written into it when the suggestion is
/// accepted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "field_definitions/")]
pub struct CreateFieldSuggestionRequest {
    pub ticket_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub field_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub draft: Option<FieldDraft>,
    /// The ticket's team…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub team_id: Option<String>,
    /// …or its type…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ticket_type_key: Option<String>,
    /// …or its project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "unknown")]
    pub value: Option<JsonValue>,
}

/// `GET /work/workspaces/:id/field-suggestions`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "field_definitions/", optional_fields)]
pub struct ListFieldSuggestionsQuery {
    /// Default `open`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<FieldSuggestionStatus>,
}

/// `POST /work/field-suggestions/:id/accept`: the rule is made (for a
/// draft first the field, with `key` or one derived from the label) and
/// every open suggestion of the entry is closed. Who decides second gets a
/// `409` ("already decided").
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "field_definitions/", optional_fields)]
pub struct AcceptFieldSuggestionRequest {
    /// A draft's key; absent: derived from its label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

/// What an accept made.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "field_definitions/")]
pub struct AcceptFieldSuggestionResponse {
    pub suggestion: FieldSuggestionResponse,
    /// The rule (an existing one when the target had it already).
    pub assignment: FieldAssignmentResponse,
    /// The field a draft became.
    pub field_id: String,
}

pub type FieldSuggestionListResponse = crate::ListResponse<FieldSuggestionResponse>;

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

/// What a ticket is, as far as its fields are concerned (§2.2).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "field_definitions/")]
pub struct FieldContext {
    pub team_id: Option<String>,
    pub ticket_type: String,
    pub project_id: Option<String>,
    /// The fields hung onto this ticket alone.
    pub adhoc_field_ids: Vec<String>,
}

/// What the resolution reads of a definition (a ticket field). A
/// [`crate::field_definitions::FieldDefinitionResponse`] has all of it
/// (`archived` = `archived_at` is set).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "field_definitions/")]
pub struct ResolutionDefinition {
    pub id: String,
    pub key: String,
    pub position: i32,
    pub archived: bool,
    #[ts(type = "unknown")]
    pub default_value: Option<JsonValue>,
}

/// What the resolution reads of a rule. A [`FieldAssignmentResponse`] has
/// all of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "field_definitions/")]
pub struct ResolutionRule {
    pub id: String,
    pub field_id: String,
    pub team_id: Option<String>,
    pub ticket_type_key: Option<String>,
    pub project_id: Option<String>,
    pub required: bool,
    pub required_on: RequiredOn,
    #[ts(type = "unknown")]
    pub default_value: Option<JsonValue>,
}

impl ResolutionRule {
    /// Every condition it sets holds for `ctx`.
    pub fn matches(&self, ctx: &FieldContext) -> bool {
        let holds = |condition: &Option<String>, actual: Option<&str>| match condition {
            None => true,
            Some(want) => actual == Some(want.as_str()),
        };
        holds(&self.team_id, ctx.team_id.as_deref())
            && holds(&self.ticket_type_key, Some(ctx.ticket_type.as_str()))
            && holds(&self.project_id, ctx.project_id.as_deref())
    }

    /// How specific it is: the conditions it sets, then project before
    /// team before type (§2.2 rule 3).
    fn specificity(&self) -> (usize, bool, bool, bool) {
        let (p, t, k) = (
            self.project_id.is_some(),
            self.team_id.is_some(),
            self.ticket_type_key.is_some(),
        );
        (usize::from(p) + usize::from(t) + usize::from(k), p, t, k)
    }
}

/// One rule a resolved field came from (for "Pflicht für Team iOS ·
/// Fehler").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "field_definitions/")]
pub struct FieldOrigin {
    pub assignment_id: String,
    pub team_id: Option<String>,
    pub ticket_type_key: Option<String>,
    pub project_id: Option<String>,
    pub required: bool,
    pub required_on: RequiredOn,
}

/// One field of a ticket, resolved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "field_definitions/")]
pub struct ResolvedField {
    pub definition_id: String,
    pub key: String,
    /// Shown in the ticket's properties: a rule matches, or it hangs on
    /// this ticket; never an archived field.
    pub visible: bool,
    /// Some matching rule requires it.
    pub required: bool,
    /// `create` when any requiring rule says create, else `done`; `null`
    /// when not required.
    pub required_on: Option<RequiredOn>,
    /// The default a create fills in: the most specific matching rule's,
    /// else the definition's.
    #[ts(type = "unknown")]
    pub default: Option<JsonValue>,
    /// The matching rules, most specific first.
    pub origins: Vec<FieldOrigin>,
    /// Hung onto this ticket alone.
    pub adhoc: bool,
    /// Not visible, but the ticket holds a value ("Weitere Werte"): kept,
    /// editable or removable, never lost (§2.2 rule 5).
    pub orphan: bool,
    /// The definition is archived (its value is read-only).
    pub archived: bool,
}

/// No value at all: `null`, an empty or blank string, an empty list.
pub fn is_blank(value: &JsonValue) -> bool {
    match value {
        JsonValue::Null => true,
        JsonValue::String(s) => s.trim().is_empty(),
        JsonValue::Array(a) => a.is_empty(),
        _ => false,
    }
}

/// The fields of a ticket in context `ctx` (§2.2):
///
/// 1. **Visible**: every definition a matching rule names, plus the ones
///    hung onto the ticket; never an archived one, whose rules rest.
/// 2. **Required** when any matching rule says so; `required_on` is
///    `create` when any requiring rule says create.
/// 3. **Default**: the most specific matching rule with a default (more
///    conditions; on a tie project, then team, then type), else the
///    definition's own.
/// 4. **Order**: rule fields by the definitions' global `position` (then
///    key), then fields of this ticket alone, then the values without a
///    field in view.
/// 5. **Values without a rule**: a stored, non-blank value of a field that
///    is not visible is an `orphan` (archived ones flagged), shown under
///    "Weitere Werte". A key no definition knows is not resolved.
///
/// `definitions` are the workspace's ticket fields, archived ones
/// included; `rules` its rules; `values` the ticket's `custom_values`.
pub fn resolve_fields(
    ctx: &FieldContext,
    definitions: &[ResolutionDefinition],
    rules: &[ResolutionRule],
    values: &Map<String, JsonValue>,
) -> Vec<ResolvedField> {
    let mut ordered: Vec<&ResolutionDefinition> = definitions.iter().collect();
    ordered.sort_by(|a, b| a.position.cmp(&b.position).then_with(|| a.key.cmp(&b.key)));
    let mut by_rule = Vec::new();
    let mut adhoc_only = Vec::new();
    let mut orphans = Vec::new();
    for definition in ordered {
        let adhoc = ctx.adhoc_field_ids.iter().any(|id| id == &definition.id);
        let mut matching: Vec<&ResolutionRule> = if definition.archived {
            Vec::new()
        } else {
            rules
                .iter()
                .filter(|r| r.field_id == definition.id && r.matches(ctx))
                .collect()
        };
        // Most specific first; equal ones by id, so every side agrees.
        matching.sort_by(|a, b| {
            b.specificity()
                .cmp(&a.specificity())
                .then_with(|| a.id.cmp(&b.id))
        });
        let required = matching.iter().any(|r| r.required);
        let required_on = if !required {
            None
        } else if matching
            .iter()
            .any(|r| r.required && r.required_on == RequiredOn::Create)
        {
            Some(RequiredOn::Create)
        } else {
            Some(RequiredOn::Done)
        };
        let default = matching
            .iter()
            .find_map(|r| r.default_value.clone())
            .or_else(|| definition.default_value.clone())
            .filter(|v| !v.is_null());
        let visible = !definition.archived && (adhoc || !matching.is_empty());
        let has_value = values.get(&definition.key).is_some_and(|v| !is_blank(v));
        let field = ResolvedField {
            definition_id: definition.id.clone(),
            key: definition.key.clone(),
            visible,
            required,
            required_on,
            default,
            origins: matching
                .iter()
                .map(|r| FieldOrigin {
                    assignment_id: r.id.clone(),
                    team_id: r.team_id.clone(),
                    ticket_type_key: r.ticket_type_key.clone(),
                    project_id: r.project_id.clone(),
                    required: r.required,
                    required_on: r.required_on,
                })
                .collect(),
            adhoc,
            orphan: !visible && has_value,
            archived: definition.archived,
        };
        if !matching.is_empty() && visible {
            by_rule.push(field);
        } else if visible {
            adhoc_only.push(field);
        } else if has_value {
            orphans.push(field);
        }
    }
    by_rule.extend(adhoc_only);
    by_rule.extend(orphans);
    by_rule
}

// ---------------------------------------------------------------------------
// Versions
// ---------------------------------------------------------------------------

/// Semver-like order of version labels (E5): split at `.`, `-`, `_`, `+`
/// and spaces; numeric parts compare as numbers, others as text (ignoring
/// case), a numeric part before a textual one, a shorter version before a
/// longer one with the same start (`17.2` < `17.2.1` < `17.10`).
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let parts = |s: &str| -> Vec<String> {
        s.split(['.', '-', '_', '+', ' '])
            .filter(|p| !p.is_empty())
            .map(str::to_ascii_lowercase)
            .collect()
    };
    let (pa, pb) = (parts(a), parts(b));
    for (x, y) in pa.iter().zip(pb.iter()) {
        let order = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(n), Ok(m)) => n.cmp(&m),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => x.cmp(y),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    pa.len().cmp(&pb.len()).then_with(|| a.cmp(b))
}

// ---------------------------------------------------------------------------
// The shared cases
// ---------------------------------------------------------------------------

/// The cases Rust and the web are both held to
/// (`src/generated/field_definitions/field-resolution.json`): the
/// concept's precedence examples and more, with **hand-written**
/// expectations, and a grid of every rule shape against every kind of
/// ticket, whose expectation is the plain reading "every set condition
/// holds".
pub mod cases {
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Map, Value as JsonValue};

    use super::{
        FieldContext, FieldOrigin, RequiredOn, ResolutionDefinition, ResolutionRule, ResolvedField,
    };

    /// One case: a ticket, the workspace's fields and rules, and what must
    /// come out.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Case {
        pub name: String,
        pub context: FieldContext,
        pub definitions: Vec<ResolutionDefinition>,
        pub rules: Vec<ResolutionRule>,
        pub values: Map<String, JsonValue>,
        pub expected: Vec<ResolvedField>,
    }

    const IOS: &str = "team-ios";
    const WEB: &str = "team-web";
    const ATLAS: &str = "project-atlas";
    const ORION: &str = "project-orion";

    fn ctx(team: Option<&str>, ticket_type: &str, project: &str, adhoc: &[&str]) -> FieldContext {
        FieldContext {
            team_id: team.map(str::to_string),
            ticket_type: ticket_type.to_string(),
            project_id: Some(project.to_string()),
            adhoc_field_ids: adhoc.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn def(id: &str, key: &str, position: i32) -> ResolutionDefinition {
        ResolutionDefinition {
            id: id.to_string(),
            key: key.to_string(),
            position,
            archived: false,
            default_value: None,
        }
    }

    fn archived(mut d: ResolutionDefinition) -> ResolutionDefinition {
        d.archived = true;
        d
    }

    fn with_default(mut d: ResolutionDefinition, value: JsonValue) -> ResolutionDefinition {
        d.default_value = Some(value);
        d
    }

    /// A rule on `field`: team, type, project.
    fn rule(
        id: &str,
        field: &str,
        team: Option<&str>,
        ticket_type: Option<&str>,
        project: Option<&str>,
    ) -> ResolutionRule {
        ResolutionRule {
            id: id.to_string(),
            field_id: field.to_string(),
            team_id: team.map(str::to_string),
            ticket_type_key: ticket_type.map(str::to_string),
            project_id: project.map(str::to_string),
            required: false,
            required_on: RequiredOn::Create,
            default_value: None,
        }
    }

    fn requiring(mut r: ResolutionRule, on: RequiredOn) -> ResolutionRule {
        r.required = true;
        r.required_on = on;
        r
    }

    fn defaulting(mut r: ResolutionRule, value: JsonValue) -> ResolutionRule {
        r.default_value = Some(value);
        r
    }

    fn origin(r: &ResolutionRule) -> FieldOrigin {
        FieldOrigin {
            assignment_id: r.id.clone(),
            team_id: r.team_id.clone(),
            ticket_type_key: r.ticket_type_key.clone(),
            project_id: r.project_id.clone(),
            required: r.required,
            required_on: r.required_on,
        }
    }

    /// A visible field from `origins` (most specific first, as written).
    fn shown(d: &ResolutionDefinition, origins: &[&ResolutionRule]) -> ResolvedField {
        ResolvedField {
            definition_id: d.id.clone(),
            key: d.key.clone(),
            visible: true,
            required: false,
            required_on: None,
            default: None,
            origins: origins.iter().map(|r| origin(r)).collect(),
            adhoc: false,
            orphan: false,
            archived: false,
        }
    }

    fn required(mut f: ResolvedField, on: RequiredOn) -> ResolvedField {
        f.required = true;
        f.required_on = Some(on);
        f
    }

    fn default_of(mut f: ResolvedField, value: JsonValue) -> ResolvedField {
        f.default = Some(value);
        f
    }

    fn adhoc(mut f: ResolvedField) -> ResolvedField {
        f.adhoc = true;
        f
    }

    fn orphan(d: &ResolutionDefinition) -> ResolvedField {
        ResolvedField {
            definition_id: d.id.clone(),
            key: d.key.clone(),
            visible: false,
            required: false,
            required_on: None,
            default: d.default_value.clone(),
            origins: Vec::new(),
            adhoc: false,
            orphan: true,
            archived: d.archived,
        }
    }

    fn values(pairs: &[(&str, JsonValue)]) -> Map<String, JsonValue> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn case(
        name: &str,
        context: FieldContext,
        definitions: Vec<ResolutionDefinition>,
        rules: Vec<ResolutionRule>,
        values: Map<String, JsonValue>,
        expected: Vec<ResolvedField>,
    ) -> Case {
        Case {
            name: name.to_string(),
            context,
            definitions,
            rules,
            values,
            expected,
        }
    }

    /// The concept's precedence table (§2.2) and the corners around it.
    pub fn examples() -> Vec<Case> {
        let mut out = Vec::new();
        let ticket = ctx(Some(IOS), "bug", ATLAS, &[]);

        // §2.2 row 1: R1 iOS-Version for team iOS, R2 for type bug,
        // required. Both match: visible, required because of R2.
        let ios_version = def("f-iosv", "ios_version", 0);
        let r1 = rule("r1", "f-iosv", Some(IOS), None, None);
        let r2 = requiring(
            rule("r2", "f-iosv", None, Some("bug"), None),
            RequiredOn::Create,
        );
        out.push(case(
            "concept: two rules match, one requires",
            ticket.clone(),
            vec![ios_version.clone()],
            vec![r1.clone(), r2.clone()],
            Map::new(),
            // Equal specificity (team vs type): team first.
            vec![required(
                shown(&ios_version, &[&r1, &r2]),
                RequiredOn::Create,
            )],
        ));

        // §2.2 row 2: R1 Gerät team iOS default iPhone; R2 team iOS + bug
        // default iPad. Two conditions beat one.
        let device = def("f-dev", "device", 1);
        let r1 = defaulting(rule("r1", "f-dev", Some(IOS), None, None), json!("iphone"));
        let r2 = defaulting(
            rule("r2", "f-dev", Some(IOS), Some("bug"), None),
            json!("ipad"),
        );
        out.push(case(
            "concept: the most specific default wins",
            ticket.clone(),
            vec![device.clone()],
            vec![r1.clone(), r2.clone()],
            Map::new(),
            vec![default_of(shown(&device, &[&r2, &r1]), json!("ipad"))],
        ));

        // §2.2 row 3: a workspace rule shows everywhere, also without team.
        let customer = def("f-cust", "customer", 2);
        let r1 = rule("r1", "f-cust", None, None, None);
        for (label, c) in [
            (
                "concept: a workspace rule shows on an iOS bug",
                ticket.clone(),
            ),
            (
                "concept: a workspace rule shows on a ticket without team",
                ctx(None, "task", ORION, &[]),
            ),
        ] {
            out.push(case(
                label,
                c,
                vec![customer.clone()],
                vec![r1.clone()],
                Map::new(),
                vec![shown(&customer, &[&r1])],
            ));
        }

        // §2.2 row 4: Severity for bugs; the ticket became a task: the value
        // stays, under "Weitere Werte".
        let severity = def("f-sev", "severity", 3);
        let r1 = rule("r1", "f-sev", None, Some("bug"), None);
        out.push(case(
            "concept: a value whose rule no longer matches is kept as an orphan",
            ctx(Some(IOS), "task", ATLAS, &[]),
            vec![severity.clone()],
            vec![r1.clone()],
            values(&[("severity", json!("critical"))]),
            vec![orphan(&severity)],
        ));
        out.push(case(
            "a field that does not apply and holds nothing is not there",
            ctx(Some(IOS), "task", ATLAS, &[]),
            vec![severity.clone()],
            vec![r1.clone()],
            values(&[("severity", json!("  "))]),
            vec![],
        ));

        // Ties: project before team before type.
        let build = def("f-build", "build", 4);
        let by_project = defaulting(
            rule("ra", "f-build", None, Some("bug"), Some(ATLAS)),
            json!("from-project"),
        );
        let by_team = defaulting(
            rule("rb", "f-build", Some(IOS), Some("bug"), None),
            json!("from-team"),
        );
        let by_type_team = defaulting(
            rule("rc", "f-build", Some(IOS), None, Some(ATLAS)),
            json!("from-project-team"),
        );
        out.push(case(
            "a tie of two conditions: project and team beats project and type beats team and type",
            ticket.clone(),
            vec![build.clone()],
            vec![by_team.clone(), by_project.clone(), by_type_team.clone()],
            Map::new(),
            vec![default_of(
                shown(&build, &[&by_type_team, &by_project, &by_team]),
                json!("from-project-team"),
            )],
        ));
        let one_project = defaulting(rule("rd", "f-build", None, None, Some(ATLAS)), json!("p"));
        let one_team = defaulting(rule("re", "f-build", Some(IOS), None, None), json!("t"));
        let one_type = defaulting(rule("rf", "f-build", None, Some("bug"), None), json!("k"));
        out.push(case(
            "a tie of one condition: project beats team beats type",
            ticket.clone(),
            vec![build.clone()],
            vec![one_type.clone(), one_team.clone(), one_project.clone()],
            Map::new(),
            vec![default_of(
                shown(&build, &[&one_project, &one_team, &one_type]),
                json!("p"),
            )],
        ));

        // A more specific rule without a default leaves the default to a
        // less specific one, then to the definition.
        let region = with_default(def("f-reg", "region", 5), json!("EU"));
        let specific = rule("r1", "f-reg", Some(IOS), Some("bug"), None);
        let general = defaulting(rule("r2", "f-reg", None, None, None), json!("US"));
        out.push(case(
            "the most specific rule that has a default gives it",
            ticket.clone(),
            vec![region.clone()],
            vec![specific.clone(), general.clone()],
            Map::new(),
            vec![default_of(
                shown(&region, &[&specific, &general]),
                json!("US"),
            )],
        ));
        out.push(case(
            "without a rule default the definition's default applies",
            ticket.clone(),
            vec![region.clone()],
            vec![specific.clone()],
            Map::new(),
            vec![default_of(shown(&region, &[&specific]), json!("EU"))],
        ));

        // Required: create beats done; done alone is done.
        let fix = def("f-fix", "fix_version", 6);
        let at_done = requiring(
            rule("r1", "f-fix", None, Some("bug"), None),
            RequiredOn::Done,
        );
        let at_create = requiring(
            rule("r2", "f-fix", Some(IOS), None, None),
            RequiredOn::Create,
        );
        out.push(case(
            "required at close only",
            ticket.clone(),
            vec![fix.clone()],
            vec![at_done.clone()],
            Map::new(),
            vec![required(shown(&fix, &[&at_done]), RequiredOn::Done)],
        ));
        out.push(case(
            "required at create by one rule and at close by another: at create",
            ticket.clone(),
            vec![fix.clone()],
            vec![at_done.clone(), at_create.clone()],
            Map::new(),
            vec![required(
                shown(&fix, &[&at_create, &at_done]),
                RequiredOn::Create,
            )],
        ));

        // No team on the ticket: a team rule does not match.
        let r1 = rule("r1", "f-dev", Some(IOS), None, None);
        out.push(case(
            "a team rule does not match a ticket without a team",
            ctx(None, "bug", ATLAS, &[]),
            vec![device.clone()],
            vec![r1.clone()],
            Map::new(),
            vec![],
        ));
        out.push(case(
            "a team rule does not match a ticket of another team",
            ctx(Some(WEB), "bug", ATLAS, &[]),
            vec![device.clone()],
            vec![r1.clone()],
            Map::new(),
            vec![],
        ));

        // Fields of one ticket: after the rule fields, by position; a field
        // both hung on and matched sits with the rule fields.
        let link = def("f-link", "test_link", 0);
        let notes = def("f-notes", "notes", 9);
        let r1 = rule("r1", "f-cust", None, None, None);
        out.push(case(
            "fields of one ticket come after the rule fields",
            ctx(Some(IOS), "bug", ATLAS, &["f-link", "f-cust"]),
            vec![customer.clone(), link.clone(), notes.clone()],
            vec![r1.clone()],
            values(&[("notes", json!("kept"))]),
            vec![
                adhoc(shown(&customer, &[&r1])),
                adhoc(shown(&link, &[])),
                orphan(&notes),
            ],
        ));

        // Archived: its rules rest, a value is a read-only orphan, without a
        // value it is gone; also when hung onto the ticket.
        let old = archived(def("f-old", "old_field", 1));
        let r1 = rule("r1", "f-old", None, None, None);
        out.push(case(
            "an archived field with a value is an archived orphan",
            ctx(Some(IOS), "bug", ATLAS, &["f-old"]),
            vec![old.clone()],
            vec![r1.clone()],
            values(&[("old_field", json!(["a"]))]),
            vec![{
                let mut f = orphan(&old);
                f.adhoc = true;
                f
            }],
        ));
        out.push(case(
            "an archived field without a value is not there",
            ctx(Some(IOS), "bug", ATLAS, &[]),
            vec![old.clone()],
            vec![r1],
            values(&[("old_field", json!([]))]),
            vec![],
        ));

        // Order: the definitions' global position, then key; a value of no
        // definition is not resolved; a rule of no definition is ignored.
        let a = def("f-a", "alpha", 2);
        let b = def("f-b", "beta", 1);
        let c = def("f-c", "gamma", 1);
        let ra = rule("ra", "f-a", None, None, None);
        let rb = rule("rb", "f-b", None, None, None);
        let rc = rule("rc", "f-c", None, None, None);
        let stray = rule("rz", "f-gone", None, None, None);
        out.push(case(
            "order by position, then key; strays are ignored",
            ticket.clone(),
            vec![a.clone(), c.clone(), b.clone()],
            vec![ra.clone(), rc.clone(), stray, rb.clone()],
            values(&[("unknown_key", json!("x")), ("alpha", json!(0))]),
            vec![shown(&b, &[&rb]), shown(&c, &[&rc]), shown(&a, &[&ra])],
        ));

        // A project rule for another project does not match.
        let r1 = rule("r1", "f-cust", None, None, Some(ORION));
        out.push(case(
            "a project rule does not match another project",
            ticket,
            vec![customer],
            vec![r1],
            values(&[("customer", json!(false))]),
            // `false` is a value: kept.
            vec![orphan(&def("f-cust", "customer", 2))],
        ));
        out
    }

    /// Every rule shape (2³ condition sets) against every kind of ticket:
    /// visible exactly when each set condition holds.
    pub fn grid() -> Vec<Case> {
        let field = def("f-grid", "grid", 0);
        let contexts = [
            ("team iOS, bug, Atlas", ctx(Some(IOS), "bug", ATLAS, &[])),
            ("team iOS, task, Atlas", ctx(Some(IOS), "task", ATLAS, &[])),
            ("team iOS, bug, Orion", ctx(Some(IOS), "bug", ORION, &[])),
            ("team Web, bug, Atlas", ctx(Some(WEB), "bug", ATLAS, &[])),
            ("no team, bug, Atlas", ctx(None, "bug", ATLAS, &[])),
            ("no team, task, Orion", ctx(None, "task", ORION, &[])),
        ];
        let mut out = Vec::new();
        for shape in 0..8u8 {
            let team = (shape & 1 != 0).then_some(IOS);
            let ticket_type = (shape & 2 != 0).then_some("bug");
            let project = (shape & 4 != 0).then_some(ATLAS);
            let r = rule("r-grid", "f-grid", team, ticket_type, project);
            for (label, c) in &contexts {
                let holds = team.is_none_or(|t| c.team_id.as_deref() == Some(t))
                    && ticket_type.is_none_or(|k| c.ticket_type == k)
                    && project.is_none_or(|p| c.project_id.as_deref() == Some(p));
                out.push(case(
                    &format!(
                        "grid: rule team={} type={} project={} on {label}",
                        team.unwrap_or("-"),
                        ticket_type.unwrap_or("-"),
                        project.unwrap_or("-"),
                    ),
                    c.clone(),
                    vec![field.clone()],
                    vec![r.clone()],
                    Map::new(),
                    if holds {
                        vec![shown(&field, &[&r])]
                    } else {
                        vec![]
                    },
                ));
            }
        }
        out
    }

    /// The cases as JSON: `{ "examples": [...], "grid": [...] }`, one case
    /// per line.
    pub fn to_json() -> Result<String, serde_json::Error> {
        let lines = |cases: Vec<Case>| -> Result<String, serde_json::Error> {
            Ok(cases
                .iter()
                .map(serde_json::to_string)
                .collect::<Result<Vec<_>, _>>()?
                .join(",\n    "))
        };
        Ok(format!(
            "{{\n  \"examples\": [\n    {}\n  ],\n  \"grid\": [\n    {}\n  ]\n}}\n",
            lines(examples())?,
            lines(grid())?
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_case_resolves_as_written() {
        let all: Vec<cases::Case> = cases::examples().into_iter().chain(cases::grid()).collect();
        assert!(all.len() > 60, "{} cases", all.len());
        for case in all {
            let got = resolve_fields(&case.context, &case.definitions, &case.rules, &case.values);
            assert_eq!(got, case.expected, "{}", case.name);
        }
    }

    #[test]
    fn the_cases_round_trip_as_json() {
        let json = cases::to_json().expect("json");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
        let examples: Vec<cases::Case> =
            serde_json::from_value(parsed["examples"].clone()).expect("examples");
        assert_eq!(examples, cases::examples());
    }

    #[test]
    fn versions_sort_like_versions() {
        let mut v = vec![
            "17.10", "17.2.1", "17.2", "beta", "16", "17.2-rc1", "2", "17.2.1",
        ];
        v.sort_by(|a, b| compare_versions(a, b));
        assert_eq!(
            v,
            // A textual part sorts after a numeric one: `17.2.1` < `17.2-rc1`.
            vec!["2", "16", "17.2", "17.2.1", "17.2.1", "17.2-rc1", "17.10", "beta"]
        );
    }

    #[test]
    fn a_rule_draft_refuses_unknown_fields() {
        assert!(serde_json::from_value::<FieldRuleDraft>(json!({ "team": "x" })).is_err());
        assert!(
            serde_json::from_value::<UpdateFieldAssignmentRequest>(json!({ "reqired": true }))
                .is_err()
        );
        let draft: FieldRuleDraft =
            serde_json::from_value(json!({ "ticket_type_key": "bug", "required_on": "done" }))
                .expect("draft");
        assert_eq!(draft.required_on, Some(RequiredOn::Done));
    }

    #[test]
    fn blank_is_null_empty_or_spaces() {
        for v in [json!(null), json!(""), json!("  "), json!([])] {
            assert!(is_blank(&v), "{v}");
        }
        for v in [json!(false), json!(0), json!("x"), json!(["a"])] {
            assert!(!is_blank(&v), "{v}");
        }
    }
}
