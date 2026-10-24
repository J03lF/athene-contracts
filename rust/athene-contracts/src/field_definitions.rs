//! Field-definition contracts: the user-defined custom-field schema layer.
//!
//! Definitions live per workspace and decorate a scope (`ticket` or
//! `project`). Each ticket carries a `custom_values` JSON object keyed by
//! the definition's stable `key`. Renaming a label is safe; renaming the
//! key is not: the service layer enforces immutability of `key`,
//! `field_type` and `applies_to` after creation.
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
}

/// One choice of a `select` or `multi_select` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "field_definitions/")]
pub struct SelectChoice {
    pub value: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Options of a `select` or `multi_select` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "field_definitions/")]
pub struct SelectFieldOptions {
    pub choices: Vec<SelectChoice>,
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
/// against the field type). The user, date, boolean and url types take an
/// empty object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "field_definitions/")]
pub enum FieldOptions {
    Select(SelectFieldOptions),
    Number(NumberFieldOptions),
    Text(TextFieldOptions),
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
    /// Required only for tickets of the types it applies to.
    pub required: bool,
    pub position: i32,
    /// Ticket-type keys the field applies to; `null` = every type (ticket
    /// fields only; always `null` for project fields). A ticket of another
    /// type does not show or require the field, and keeps any value it
    /// already has, so changing a ticket's type loses nothing.
    #[serde(default)]
    pub ticket_type_keys: Option<Vec<String>>,
    /// Filled in on create when the field applies and no value is sent.
    /// Same shape as a value of this field (validated like one).
    #[serde(default)]
    #[ts(type = "unknown")]
    pub default_value: Option<JsonValue>,
    /// Shown under the field. ≤ 200 chars.
    #[serde(default)]
    pub help_text: Option<String>,
    /// Show the value on board cards and list rows.
    #[serde(default)]
    pub show_on_card: bool,
    /// Input placeholder. ≤ 80 chars.
    #[serde(default)]
    pub placeholder: Option<String>,
    pub created_at: String,
    pub updated_at: String,
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
    #[serde(default)]
    #[ts(optional, as = "Option<bool>")]
    pub required: bool,
    /// Tail by default: the server places new fields at the end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub position: Option<i32>,
    /// Absent, `null` or `[]` = every type. Keys must be types of the
    /// workspace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket_type_keys: Option<Vec<String>>,
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
/// it is; `null` clears it (for `ticket_type_keys`: back to every type).
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema, Validate, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "field_definitions/")]
pub struct UpdateFieldDefinitionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1, max = 80))]
    #[ts(optional)]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, as = "Option<FieldOptions>")]
    pub options: Option<JsonValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub required: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub position: Option<i32>,
    /// `null` or `[]` = every type.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<Vec<String>>)]
    pub ticket_type_keys: Patch<Vec<String>>,
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
            json!({ "ticket_type_keys": ["bug"], "default_value": { "a": 1 } }),
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
