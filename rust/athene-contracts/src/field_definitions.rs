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
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
pub struct UpdateFieldDefinitionRequest {
    #[validate(length(min = 1, max = 80))]
    pub label: Option<String>,
    pub options: Option<JsonValue>,
    pub required: Option<bool>,
    pub position: Option<i32>,
}

/// Bulk-reorder definitions within a workspace + scope. Server applies the
/// listed positions in one transaction so the order can't tear half-way.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ReorderFieldDefinitionsRequest {
    pub applies_to: AppliesTo,
    pub ordered_ids: Vec<String>,
}

pub type FieldDefinitionListResponse = crate::ListResponse<FieldDefinitionResponse>;
