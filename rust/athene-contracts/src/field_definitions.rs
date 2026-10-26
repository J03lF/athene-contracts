//! Field-definition contracts: the user-defined custom-field schema layer.
//!
//! Definitions live per workspace and decorate a scope (`ticket` or
//! `project`). Each ticket carries a `custom_values` JSON object keyed by
//! the definition's stable `key`. Renaming a label is safe; renaming the
//! key is not: the service layer enforces immutability of `key`,
//! `field_type` and `applies_to` after creation.
//!
//! Where a ticket field applies (team, ticket type, project and their
//! combinations), whether it is required there and its default there are
//! rules of their own since custom fields wave 3 (CF1,
//! `artifacts/custom-fields/CONCEPT.md` §2.2): see
//! [`crate::field_assignments`]. A definition says **what** a field is,
//! not **where** it applies. Project fields (`applies_to = project`) keep
//! their `required` flag and take no rules.
//!
//! The gateway parses request bodies with these types before forwarding,
//! the core reads the update with them, and the TypeScript in
//! `src/generated/field_definitions/` is generated from them (see
//! `tests/ts_bindings.rs`). Request bodies refuse unknown fields.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use ts_rs::TS;
use utoipa::ToSchema;
use validator::Validate;

use crate::edit::SetOps;
use crate::field_assignments::FieldRuleDraft;
use crate::Patch;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "field_definitions/")]
pub enum AppliesTo {
    Ticket,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "field_definitions/")]
pub enum FieldType {
    Text,
    LongText,
    Number,
    Select,
    MultiSelect,
    User,
    Date,
    Boolean,
    Url,
    /// A point in time, `"2026-10-25T14:30:00Z"` (UTC; shown in the
    /// reader's time zone).
    Datetime,
    /// One of the field's own ordered versions (`options.choices`, oldest
    /// first; E5), e.g. `"17.2.1"`. With `options.allow_new` a ticket may
    /// add one.
    Version,
}

impl FieldType {
    /// Every type, in the settings' order.
    pub const ALL: [FieldType; 11] = [
        FieldType::Text,
        FieldType::LongText,
        FieldType::Number,
        FieldType::Select,
        FieldType::MultiSelect,
        FieldType::User,
        FieldType::Date,
        FieldType::Datetime,
        FieldType::Boolean,
        FieldType::Url,
        FieldType::Version,
    ];

    /// The wire name.
    pub const fn as_str(self) -> &'static str {
        match self {
            FieldType::Text => "text",
            FieldType::LongText => "long_text",
            FieldType::Number => "number",
            FieldType::Select => "select",
            FieldType::MultiSelect => "multi_select",
            FieldType::User => "user",
            FieldType::Date => "date",
            FieldType::Boolean => "boolean",
            FieldType::Url => "url",
            FieldType::Datetime => "datetime",
            FieldType::Version => "version",
        }
    }

    /// The type by its wire name.
    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.as_str() == raw)
    }
}

/// One choice of a `select`, `multi_select` or `version` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "field_definitions/")]
pub struct SelectChoice {
    pub value: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// No longer offered; a value that holds it keeps its label (L11).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ts(optional, as = "Option<bool>")]
    pub archived: bool,
}

/// Options of a `select` or `multi_select` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "field_definitions/")]
pub struct SelectFieldOptions {
    pub choices: Vec<SelectChoice>,
}

/// Options of a `version` field (E5): the versions in order, oldest first.
/// The order is the field's sort order, wherever it is sorted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "field_definitions/")]
pub struct VersionFieldOptions {
    /// May be empty: the first version can come from a ticket.
    pub choices: Vec<SelectChoice>,
    /// A ticket may add a version that is not in the list yet (`ticket.edit`
    /// is enough); it joins the list in its semver-like place.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ts(optional, as = "Option<bool>")]
    pub allow_new: bool,
}

/// Options of a `user` field.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "field_definitions/")]
pub struct UserFieldOptions {
    /// Several people: the value is a list of user ids, merged as a set.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ts(optional, as = "Option<bool>")]
    pub multiple: bool,
}

/// Options of a `number` field.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "field_definitions/")]
pub struct NumberFieldOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub step: Option<f64>,
    /// Shown after the value ("ms", "€"); display only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub unit: Option<String>,
}

/// Options of a `text` or `long_text` field.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "field_definitions/")]
pub struct TextFieldOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub max_length: Option<u32>,
}

/// The type-specific options object, by field type. Describes the JSON the
/// core validates before accepting a definition, so renderers can trust
/// it; on the Rust side the field stays a JSON value (the core checks it
/// against the field type). The date, datetime, boolean and url types take
/// an empty object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "field_definitions/")]
pub enum FieldOptions {
    Version(VersionFieldOptions),
    Select(SelectFieldOptions),
    Number(NumberFieldOptions),
    Text(TextFieldOptions),
    User(UserFieldOptions),
    Empty(#[ts(type = "Record<string, never>")] Map<String, JsonValue>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export_to = "field_definitions/")]
pub struct FieldDefinitionResponse {
    pub id: String,
    pub workspace_id: String,
    pub applies_to: AppliesTo,
    /// Stable machine key, `^[a-z][a-z0-9_]{0,31}$`. Immutable after
    /// creation; the value bucket on each ticket is keyed against this.
    pub key: String,
    pub label: String,
    pub field_type: FieldType,
    /// Type-specific options; see [`FieldOptions`].
    #[ts(as = "FieldOptions")]
    pub options: JsonValue,
    /// Project fields only: the project must carry a value. Always `false`
    /// for ticket fields, whose requirement is per rule
    /// ([`crate::field_assignments::FieldAssignmentResponse::required`]).
    pub required: bool,
    /// One global order for every ticket view (CONCEPT §2.2 rule 4).
    pub position: i32,
    /// The base default: filled in on create when the field applies, no
    /// value is sent and no rule names a default of its own. Same shape as
    /// a value of this field, or one of the tokens `"$me"` (user), `"$today"`
    /// (date), `"$now"` (datetime), `"$latest"` (version), resolved on
    /// create.
    #[serde(default)]
    #[ts(type = "unknown")]
    pub default_value: Option<JsonValue>,
    /// One line for the settings list. ≤ 200 chars.
    #[serde(default)]
    pub description: Option<String>,
    /// Shown under the field. ≤ 200 chars.
    #[serde(default)]
    pub help_text: Option<String>,
    /// Show the value on board cards and list rows.
    #[serde(default)]
    pub show_on_card: bool,
    /// Input placeholder. ≤ 80 chars.
    #[serde(default)]
    pub placeholder: Option<String>,
    /// Set while the field is archived (§2.6): not offered anywhere new,
    /// its rules rest, its values stay readable. RFC 3339.
    #[serde(default)]
    pub archived_at: Option<String>,
    /// Rises with every write to the definition (3.7); orders answers.
    pub version: i64,
    /// Tickets (projects, for a project field) that hold a value. Only for
    /// whoever may manage fields, and only when asked for (`with_usage`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub usage_count: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

/// `GET /work/workspaces/:id/field-definitions`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export_to = "field_definitions/", optional_fields)]
pub struct ListFieldDefinitionsQuery {
    /// Archived definitions too (the settings' "Archiviert" section, and a
    /// ticket's archived values). Default: only active ones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_archived: Option<bool>,
    /// Count the values per definition (`usage_count`); answered only for
    /// whoever may manage fields, ignored for everyone else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_usage: Option<bool>,
}

/// `POST /work/workspaces/:id/field-definitions`.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "field_definitions/")]
pub struct CreateFieldDefinitionRequest {
    pub applies_to: AppliesTo,
    #[validate(length(min = 1, max = 32))]
    pub key: String,
    #[validate(length(min = 1, max = 80))]
    pub label: String,
    pub field_type: FieldType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<FieldOptions>")]
    pub options: Option<JsonValue>,
    /// A project field: required on every project. A ticket field without
    /// `rules`: copied into the rules made from `ticket_type_keys`, as
    /// phase 1 did (and as migration 0038 did for the fields of then).
    #[serde(default)]
    #[ts(optional, as = "Option<bool>")]
    pub required: bool,
    /// Tail by default: the server places new fields at the end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub position: Option<i32>,
    /// A ticket field without `rules`: absent, `null` or `[]` makes one
    /// rule for the whole workspace, otherwise one rule per type (phase-1
    /// semantics). Keys must be types of the workspace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket_type_keys: Option<Vec<String>>,
    /// A ticket field's rules, made with it (`[]`: none, e.g. a field for
    /// one ticket only). Excludes `ticket_type_keys` and `required`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub rules: Option<Vec<FieldRuleDraft>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "unknown")]
    pub default_value: Option<JsonValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub show_on_card: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
}

/// `PATCH /work/field-definitions/:id`. An absent key leaves that part as
/// it is; `null` clears it.
///
/// Where a ticket field applies and whether it is required there are its
/// rules (`/work/field-assignments`); `required` here is a project field's.
/// With `expected`, each named part is checked against what the client
/// saw: a part that moved since is a `409` `ATH-RES-003` (`field` = its
/// name), and nothing is written. `choices` adds, archives and restores
/// choices as set operations, without a base: two managers adding choices
/// at once both keep theirs (CONCEPT §4.3).
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema, Validate, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "field_definitions/")]
pub struct UpdateFieldDefinitionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1, max = 80))]
    #[ts(optional)]
    pub label: Option<String>,
    /// The whole options object (choices included).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<FieldOptions>")]
    pub options: Option<JsonValue>,
    /// Project fields only (a ticket field: `400`, its rules say it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub required: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub position: Option<i32>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub description: Patch<String>,
    /// Choice operations on `options.choices` (select, multi-select,
    /// version). Not together with `options`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    #[schema(value_type = Option<Object>)]
    pub choices: Option<ChoiceOps>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    #[schema(value_type = Option<Object>)]
    pub expected: Option<FieldDefinitionExpected>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<Object>)]
    #[ts(type = "unknown")]
    pub default_value: Patch<JsonValue>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub help_text: Patch<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub show_on_card: Option<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub placeholder: Patch<String>,
}

/// Choice operations of a definition update: what to add (a `value` that is
/// there already is left as it is), which values to archive, which to
/// bring back. A value in more than one list is a `400`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "field_definitions/")]
pub struct ChoiceOps {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<SelectChoice>>")]
    pub add: Vec<SelectChoice>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<String>>")]
    pub archive: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<String>>")]
    pub restore: Vec<String>,
}

impl ChoiceOps {
    pub fn is_empty(&self) -> bool {
        self.add.is_empty() && self.archive.is_empty() && self.restore.is_empty()
    }

    /// The operations as plain set operations on the values (for the
    /// contradiction check): added and restored values, archived values.
    pub fn as_set_ops(&self) -> SetOps<String> {
        SetOps {
            add: self
                .add
                .iter()
                .map(|c| c.value.clone())
                .chain(self.restore.iter().cloned())
                .collect(),
            remove: self.archive.clone(),
        }
    }
}

/// The base of a definition update: per part the client changes, what it
/// saw (`null`: was empty). Absent parts are not checked.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "field_definitions/")]
pub struct FieldDefinitionExpected {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<FieldOptions>")]
    pub options: Option<JsonValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub required: Option<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub description: Patch<String>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[ts(type = "unknown")]
    pub default_value: Patch<JsonValue>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub help_text: Patch<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub show_on_card: Option<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    pub placeholder: Patch<String>,
}

/// Bulk-reorder definitions within a workspace + scope
/// (`POST /work/workspaces/:id/field-definitions/reorder`). The server
/// applies the listed positions in one go so the order cannot tear
/// half-way.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "field_definitions/")]
pub struct ReorderFieldDefinitionsRequest {
    pub applies_to: AppliesTo,
    pub ordered_ids: Vec<String>,
}

pub type FieldDefinitionListResponse = crate::ListResponse<FieldDefinitionResponse>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_update_tells_absent_from_null_and_forwards_as_sent() {
        for body in [
            json!({ "label": "Severity" }),
            json!({ "help_text": null, "default_value": null }),
            json!({ "description": "x", "default_value": { "a": 1 } }),
            json!({ "choices": { "add": [{ "value": "a", "label": "A" }], "archive": ["b"] },
                    "expected": { "label": "Severity", "description": null } }),
        ] {
            let req: UpdateFieldDefinitionRequest =
                serde_json::from_value(body.clone()).expect("patch");
            assert_eq!(serde_json::to_value(&req).expect("serialise"), body);
        }
    }

    #[test]
    fn requests_refuse_unknown_fields() {
        assert!(
            serde_json::from_value::<UpdateFieldDefinitionRequest>(json!({ "lable": "x" }))
                .is_err()
        );
        assert!(
            serde_json::from_value::<CreateFieldDefinitionRequest>(json!({
                "applies_to": "ticket", "key": "k", "label": "K", "field_type": "text",
                "requried": true
            }))
            .is_err()
        );
    }

    #[test]
    fn every_field_type_reads_back_its_wire_name() {
        for t in FieldType::ALL {
            let wire = serde_json::to_value(t).expect("serialise");
            assert_eq!(wire, json!(t.as_str()));
            assert_eq!(FieldType::parse(t.as_str()), Some(t));
        }
        assert_eq!(FieldType::parse("nope"), None);
    }

    #[test]
    fn an_archived_choice_says_so_and_an_active_one_does_not() {
        let active: SelectChoice =
            serde_json::from_value(json!({ "value": "a", "label": "A" })).expect("choice");
        assert!(!active.archived);
        assert_eq!(
            serde_json::to_value(&active).expect("serialise"),
            json!({ "value": "a", "label": "A" })
        );
        let old: SelectChoice =
            serde_json::from_value(json!({ "value": "b", "label": "B", "archived": true }))
                .expect("choice");
        assert!(old.archived);
    }

    #[test]
    fn a_create_forwards_only_what_was_sent() {
        let body = json!({
            "applies_to": "ticket", "key": "k", "label": "K", "field_type": "select",
            "options": { "choices": [{ "value": "a", "label": "A" }] },
            "ticket_type_keys": ["bug"]
        });
        let req: CreateFieldDefinitionRequest = serde_json::from_value(body).expect("create");
        let forwarded = serde_json::to_value(&req).expect("serialise");
        assert_eq!(forwarded["required"], json!(false));
        assert!(forwarded.get("position").is_none());
        assert!(forwarded.get("help_text").is_none());
    }
}
