/**
 * Conflict-aware edits (step 3.7, `CONFLICTS.md`): set operations, the
 * long-text hash and the `details` of a `409` `ATH-RES-003`.
 *
 * Generated from `rust/athenos-contracts/src/edit.rs` by ts-rs; do not add
 * hand-written wire shapes here.
 */
export type { ChangedBy } from '../generated/edit/ChangedBy';
export type { EditConflictDetails } from '../generated/edit/EditConflictDetails';
export type { FieldConflict } from '../generated/edit/FieldConflict';
export type { SetOps } from '../generated/edit/SetOps';
export type { TextHash } from '../generated/edit/TextHash';

/** `error.code` of an edit whose base no longer holds. */
export const EDIT_CONFLICT_CODE = 'ATH-RES-003';
/** The i18n key a client renders for {@link EDIT_CONFLICT_CODE}. */
export const EDIT_CONFLICT_MESSAGE_KEY = 'errors.edit_conflict';
