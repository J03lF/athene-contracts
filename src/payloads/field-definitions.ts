/**
 * Custom field-definition wire types, generated from
 * `rust/athene-contracts/src/field_definitions.rs` by ts-rs.
 *
 * Definitions live per workspace and decorate either tickets or projects.
 * Each ticket carries a `custom_values` JSON object keyed by the
 * definition's stable `key`. Renaming the label is safe; renaming the key
 * is not: the server rejects key changes after creation.
 */
export type { AppliesTo } from '../generated/field_definitions/AppliesTo';
export type { CreateFieldDefinitionRequest } from '../generated/field_definitions/CreateFieldDefinitionRequest';
export type { FieldDefinitionResponse } from '../generated/field_definitions/FieldDefinitionResponse';
export type { FieldOptions } from '../generated/field_definitions/FieldOptions';
export type { FieldType } from '../generated/field_definitions/FieldType';
export type { NumberFieldOptions } from '../generated/field_definitions/NumberFieldOptions';
export type { ReorderFieldDefinitionsRequest } from '../generated/field_definitions/ReorderFieldDefinitionsRequest';
export type { SelectChoice } from '../generated/field_definitions/SelectChoice';
export type { SelectFieldOptions } from '../generated/field_definitions/SelectFieldOptions';
export type { TextFieldOptions } from '../generated/field_definitions/TextFieldOptions';
export type { UpdateFieldDefinitionRequest } from '../generated/field_definitions/UpdateFieldDefinitionRequest';
