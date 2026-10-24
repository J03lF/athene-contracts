//! Field-definition DTOs — the user-defined custom-field schema layer.
//!
//! Definitions live per workspace and decorate a scope (`ticket`, later
//! `project`). Each ticket carries a `custom_values` JSON object keyed by
//! the definition's stable `key`. Renaming a label is safe; renaming the
//! key is not — the service layer enforces immutability of `key`,
//! `field_type`, and `applies_to` after creation.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;
use validator::Validate;

use crate::Patch;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AppliesTo {
    Ticket,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct FieldDefinitionResponse {
    pub id: String,
    pub workspace_id: String,
    pub applies_to: AppliesTo,
    /// Stable machine key — `^[a-z][a-z0-9_]{0,31}$`. Immutable after
    /// creation; the value bucket on each ticket is keyed against this.
    pub key: String,
    pub label: String,
    pub field_type: FieldType,
    /// Type-specific options object. See server-side `FieldDefinition::new`
    /// for the shape per field type.
    pub options: JsonValue,
    pub required: bool,
    pub position: i32,
    /// Ticket-type keys the field applies to; `None` = every type.
    #[serde(default)]
    pub ticket_type_keys: Option<Vec<String>>,
    /// Filled in on create when the field applies and no value is sent.
    #[serde(default)]
    pub default_value: Option<JsonValue>,
    #[serde(default)]
    pub help_text: Option<String>,
    #[serde(default)]
    pub show_on_card: bool,
    #[serde(default)]
    pub placeholder: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
pub struct CreateFieldDefinitionRequest {
    pub applies_to: AppliesTo,
    #[validate(length(min = 1, max = 32))]
    pub key: String,
    #[validate(length(min = 1, max = 80))]
    pub label: String,
    pub field_type: FieldType,
    pub options: Option<JsonValue>,
    #[serde(default)]
    pub required: bool,
    /// Tail by default — server places new fields at the end.
    pub position: Option<i32>,
    /// `None` or empty = every type.
    #[serde(default)]
    pub ticket_type_keys: Option<Vec<String>>,
    #[serde(default)]
    pub default_value: Option<JsonValue>,
    #[serde(default)]
    pub help_text: Option<String>,
    #[serde(default)]
    pub show_on_card: Option<bool>,
    #[serde(default)]
    pub placeholder: Option<String>,
}

/// `PATCH /field-definitions/:id`. An absent key leaves that part as it
/// is; `null` clears it (for `ticket_type_keys`: back to every type).
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema, Validate)]
pub struct UpdateFieldDefinitionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1, max = 80))]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<JsonValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
    /// `null` or empty = every type.
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<Vec<String>>)]
    pub ticket_type_keys: Patch<Vec<String>>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<Object>)]
    pub default_value: Patch<JsonValue>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub help_text: Patch<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_on_card: Option<bool>,
    #[serde(default, skip_serializing_if = "Patch::is_absent")]
    #[schema(value_type = Option<String>)]
    pub placeholder: Patch<String>,
}

/// Bulk-reorder definitions within a workspace + scope. Server applies the
/// listed positions in one transaction so the order can't tear half-way.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ReorderFieldDefinitionsRequest {
    pub applies_to: AppliesTo,
    pub ordered_ids: Vec<String>,
}

pub type FieldDefinitionListResponse = crate::ListResponse<FieldDefinitionResponse>;
