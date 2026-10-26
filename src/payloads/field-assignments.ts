/**
 * Where a ticket field applies (custom fields wave 3, CF1): rules per
 * team, ticket type and project, fields of one ticket, suggestions, and
 * the resolution — generated types from
 * `rust/athene-contracts/src/field_assignments.rs` by ts-rs, and
 * `resolveFields`, a TypeScript port of `field_assignments::resolve_fields`
 * held to the same cases as the Rust one (`FIELD_RESOLUTION_CASES`,
 * `src/generated/field_definitions/field-resolution.json`).
 *
 * 1. Visible: every definition a matching rule names (all its set
 *    conditions hold), plus the ones hung onto the ticket; never an
 *    archived one.
 * 2. Required when any matching rule says so (`create` beats `done`).
 * 3. Default: the most specific matching rule with a default (more
 *    conditions; on a tie project, team, type), else the definition's.
 * 4. Order: rule fields by position then key, then the ticket's own
 *    fields, then values without a visible field ("Weitere Werte").
 */
import type { FieldDefinitionResponse } from '../generated/field_definitions/FieldDefinitionResponse';
import type { FieldContext } from '../generated/field_definitions/FieldContext';
import type { FieldOrigin } from '../generated/field_definitions/FieldOrigin';
import type { RequiredOn } from '../generated/field_definitions/RequiredOn';
import type { ResolutionDefinition } from '../generated/field_definitions/ResolutionDefinition';
import type { ResolutionRule } from '../generated/field_definitions/ResolutionRule';
import type { ResolvedField } from '../generated/field_definitions/ResolvedField';

export type { AcceptFieldSuggestionRequest } from '../generated/field_definitions/AcceptFieldSuggestionRequest';
export type { AcceptFieldSuggestionResponse } from '../generated/field_definitions/AcceptFieldSuggestionResponse';
export type { AttachTicketFieldRequest } from '../generated/field_definitions/AttachTicketFieldRequest';
export type { CreateFieldAssignmentRequest } from '../generated/field_definitions/CreateFieldAssignmentRequest';
export type { CreateFieldSuggestionRequest } from '../generated/field_definitions/CreateFieldSuggestionRequest';
export type { DetachTicketFieldQuery } from '../generated/field_definitions/DetachTicketFieldQuery';
export type { FieldAssignmentExpected } from '../generated/field_definitions/FieldAssignmentExpected';
export type { FieldAssignmentResponse } from '../generated/field_definitions/FieldAssignmentResponse';
export type { FieldContext } from '../generated/field_definitions/FieldContext';
export type { FieldDraft } from '../generated/field_definitions/FieldDraft';
export type { FieldOrigin } from '../generated/field_definitions/FieldOrigin';
export type { FieldRuleDraft } from '../generated/field_definitions/FieldRuleDraft';
export type { FieldSuggestionResponse } from '../generated/field_definitions/FieldSuggestionResponse';
export type { FieldSuggestionStatus } from '../generated/field_definitions/FieldSuggestionStatus';
export type { ListFieldSuggestionsQuery } from '../generated/field_definitions/ListFieldSuggestionsQuery';
export type { RequiredOn } from '../generated/field_definitions/RequiredOn';
export type { ResolutionDefinition } from '../generated/field_definitions/ResolutionDefinition';
export type { ResolutionRule } from '../generated/field_definitions/ResolutionRule';
export type { ResolvedField } from '../generated/field_definitions/ResolvedField';
export type { UpdateFieldAssignmentRequest } from '../generated/field_definitions/UpdateFieldAssignmentRequest';
/** The shared cases (for specs; tree-shaken out of an app that does not import it). */
export {
  FIELD_RESOLUTION_CASES,
  type FieldResolutionCase,
} from '../generated/field_definitions/field-resolution';

/** The types a suggested (drafted) field may have (`FieldDraft::TYPES`). */
export const FIELD_DRAFT_TYPES = [
  'text',
  'long_text',
  'number',
  'date',
  'datetime',
  'boolean',
  'url',
  'user',
] as const;

/** No value at all: `null`/absent, an empty or blank string, an empty list. */
export function isBlankFieldValue(value: unknown): boolean {
  if (value === null || value === undefined) return true;
  if (typeof value === 'string') return value.trim() === '';
  if (Array.isArray(value)) return value.length === 0;
  return false;
}

/** What the resolution reads of a definition. */
export function toResolutionDefinition(d: FieldDefinitionResponse): ResolutionDefinition {
  return {
    id: d.id,
    key: d.key,
    position: d.position,
    archived: d.archived_at !== null && d.archived_at !== undefined,
    default_value: d.default_value ?? null,
  };
}

/** Byte-wise, as Rust compares strings (ids and keys are ASCII). */
function cmp(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

function holds(condition: string | null, actual: string | null): boolean {
  return condition === null || condition === actual;
}

/** Every condition `rule` sets holds for `ctx`. */
export function ruleMatches(rule: ResolutionRule, ctx: FieldContext): boolean {
  return (
    holds(rule.team_id, ctx.team_id) &&
    holds(rule.ticket_type_key, ctx.ticket_type) &&
    holds(rule.project_id, ctx.project_id)
  );
}

/** Conditions set, then project, team, type (larger = more specific). */
function specificity(r: ResolutionRule): number[] {
  const p = r.project_id !== null ? 1 : 0;
  const t = r.team_id !== null ? 1 : 0;
  const k = r.ticket_type_key !== null ? 1 : 0;
  return [p + t + k, p, t, k];
}

function bySpecificityDesc(a: ResolutionRule, b: ResolutionRule): number {
  const sa = specificity(a);
  const sb = specificity(b);
  for (let i = 0; i < sa.length; i++) {
    if (sa[i] !== sb[i]) return sb[i] - sa[i];
  }
  return cmp(a.id, b.id);
}

/** The fields of a ticket in context `ctx` (`resolve_fields`). */
export function resolveFields(
  ctx: FieldContext,
  definitions: ReadonlyArray<ResolutionDefinition>,
  rules: ReadonlyArray<ResolutionRule>,
  values: Readonly<Record<string, unknown>>
): ResolvedField[] {
  const ordered = [...definitions].sort((a, b) => a.position - b.position || cmp(a.key, b.key));
  const byRule: ResolvedField[] = [];
  const adhocOnly: ResolvedField[] = [];
  const orphans: ResolvedField[] = [];
  for (const definition of ordered) {
    const adhoc = ctx.adhoc_field_ids.includes(definition.id);
    const matching = definition.archived
      ? []
      : rules
          .filter((r) => r.field_id === definition.id && ruleMatches(r, ctx))
          .sort(bySpecificityDesc);
    const required = matching.some((r) => r.required);
    let requiredOn: RequiredOn | null = null;
    if (required) {
      requiredOn = matching.some((r) => r.required && r.required_on === 'create')
        ? 'create'
        : 'done';
    }
    let fallback: unknown = null;
    for (const r of matching) {
      if (r.default_value !== null && r.default_value !== undefined) {
        fallback = r.default_value;
        break;
      }
    }
    if (fallback === null) fallback = definition.default_value ?? null;
    const visible = !definition.archived && (adhoc || matching.length > 0);
    const hasValue = !isBlankFieldValue(values[definition.key]);
    const field: ResolvedField = {
      definition_id: definition.id,
      key: definition.key,
      visible,
      required,
      required_on: requiredOn,
      default: fallback,
      origins: matching.map(
        (r): FieldOrigin => ({
          assignment_id: r.id,
          team_id: r.team_id,
          ticket_type_key: r.ticket_type_key,
          project_id: r.project_id,
          required: r.required,
          required_on: r.required_on,
        })
      ),
      adhoc,
      orphan: !visible && hasValue,
      archived: definition.archived,
    };
    if (matching.length > 0 && visible) byRule.push(field);
    else if (visible) adhocOnly.push(field);
    else if (hasValue) orphans.push(field);
  }
  return [...byRule, ...adhocOnly, ...orphans];
}
