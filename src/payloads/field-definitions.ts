/**
 * Custom field-definition wire shapes.
 *
 * Definitions live per workspace and decorate either tickets or projects
 * (more scopes can be added without a new table). Each ticket carries a
 * `custom_values` JSON object keyed by the definition's stable `key`.
 * Renaming the label is safe; renaming the key is not — the server
 * rejects key changes after creation.
 */

export type AppliesTo = 'ticket' | 'project';

export type FieldType =
  | 'text'
  | 'long_text'
  | 'number'
  | 'select'
  | 'multi_select'
  | 'user'
  | 'date'
  | 'boolean'
  | 'url';

/**
 * Type-specific options shape. Each shape is what the server validates
 * before accepting a definition; downstream renderers can trust it.
 */
export interface SelectFieldOptions {
  choices: Array<{ value: string; label: string; color?: string | null }>;
}

export interface NumberFieldOptions {
  min?: number;
  max?: number;
  step?: number;
}

export interface TextFieldOptions {
  max_length?: number;
}

export type FieldOptions =
  | SelectFieldOptions
  | NumberFieldOptions
  | TextFieldOptions
  | Record<string, never>; // empty object for user/date/boolean/url

export interface FieldDefinitionResponse {
  id: string;
  workspace_id: string;
  applies_to: AppliesTo;
  /** Stable machine key — `^[a-z][a-z0-9_]{0,31}$`. Immutable. */
  key: string;
  label: string;
  field_type: FieldType;
  options: FieldOptions;
  /** Required only for tickets of the types it applies to. */
  required: boolean;
  position: number;
  /**
   * Ticket-type keys this field applies to; `null` = every type. (Ticket
   * fields only; always `null` for project fields.) A ticket of another
   * type does not show or require the field — and keeps any value it
   * already has, so changing a ticket's type loses nothing.
   */
  ticket_type_keys: string[] | null;
  /**
   * Filled in on create when the field applies and no value is sent.
   * Same shape as a value of this field (validated like one).
   */
  default_value: unknown | null;
  /** Shown under the field. ≤ 200 chars. */
  help_text: string | null;
  /** Show the value on board cards and list rows. */
  show_on_card: boolean;
  /** Input placeholder. ≤ 80 chars. */
  placeholder: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateFieldDefinitionRequest {
  applies_to: AppliesTo;
  key: string;
  label: string;
  field_type: FieldType;
  options?: FieldOptions;
  required?: boolean;
  /** Tail by default — server places new fields at the end. */
  position?: number;
  /** Absent, `null` or `[]` = every type. Keys must be types of the workspace. */
  ticket_type_keys?: string[] | null;
  default_value?: unknown | null;
  help_text?: string | null;
  show_on_card?: boolean;
  placeholder?: string | null;
}

/** An absent key leaves that part as it is; `null` clears it. */
export interface UpdateFieldDefinitionRequest {
  label?: string;
  options?: FieldOptions;
  required?: boolean;
  position?: number;
  /** `null` or `[]` = every type. */
  ticket_type_keys?: string[] | null;
  default_value?: unknown | null;
  help_text?: string | null;
  show_on_card?: boolean;
  placeholder?: string | null;
}

export interface ReorderFieldDefinitionsRequest {
  applies_to: AppliesTo;
  ordered_ids: string[];
}
