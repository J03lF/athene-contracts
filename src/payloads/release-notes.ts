/**
 * Release notes (W4-H): a template per workspace, per release exclusions,
 * order, intro/outro and an editable draft per variant (internal, public),
 * and a "Release-Notiz" per ticket. Markdown, and HTML by an allowlist.
 *
 * Generated from `rust/athene-contracts/src/release_notes.rs` by ts-rs.
 */
export type { ExclusionOps } from '../generated/release_notes/ExclusionOps';
export type { ExclusionScope } from '../generated/release_notes/ExclusionScope';
export type { GenerateReleaseNotesRequest } from '../generated/release_notes/GenerateReleaseNotesRequest';
export type { GenerateReleaseNotesResponse } from '../generated/release_notes/GenerateReleaseNotesResponse';
export type { ReleaseNoteEntry } from '../generated/release_notes/ReleaseNoteEntry';
export type { ReleaseNoteExclusion } from '../generated/release_notes/ReleaseNoteExclusion';
export type { ReleaseNoteFields } from '../generated/release_notes/ReleaseNoteFields';
export type { ReleaseNoteLanguage } from '../generated/release_notes/ReleaseNoteLanguage';
export type { ReleaseNoteSection } from '../generated/release_notes/ReleaseNoteSection';
export type { ReleaseNoteSectionOrder } from '../generated/release_notes/ReleaseNoteSectionOrder';
export type { ReleaseNoteTemplate } from '../generated/release_notes/ReleaseNoteTemplate';
export type { ReleaseNoteTemplateDefaults } from '../generated/release_notes/ReleaseNoteTemplateDefaults';
export type { ReleaseNoteTemplateResponse } from '../generated/release_notes/ReleaseNoteTemplateResponse';
export type { ReleaseNotesDraftResponse } from '../generated/release_notes/ReleaseNotesDraftResponse';
export type { ReleaseNotesExpected } from '../generated/release_notes/ReleaseNotesExpected';
export type { ReleaseNotesFileResponse } from '../generated/release_notes/ReleaseNotesFileResponse';
export type { ReleaseNotesFormat } from '../generated/release_notes/ReleaseNotesFormat';
export type { ReleaseNotesQuery } from '../generated/release_notes/ReleaseNotesQuery';
export type { ReleaseNotesResponse } from '../generated/release_notes/ReleaseNotesResponse';
export type { ReleaseNotesSectionView } from '../generated/release_notes/ReleaseNotesSectionView';
export type { ReleaseNotesSettingsResponse } from '../generated/release_notes/ReleaseNotesSettingsResponse';
export type { ReleaseNotesVariant } from '../generated/release_notes/ReleaseNotesVariant';
export type { SectionMatch } from '../generated/release_notes/SectionMatch';
export type { TicketReleaseNoteExpected } from '../generated/release_notes/TicketReleaseNoteExpected';
export type { TicketReleaseNoteResponse } from '../generated/release_notes/TicketReleaseNoteResponse';
export type { UpdateReleaseNoteTemplateRequest } from '../generated/release_notes/UpdateReleaseNoteTemplateRequest';
export type { UpdateReleaseNotesRequest } from '../generated/release_notes/UpdateReleaseNotesRequest';
export type { UpdateTicketReleaseNoteRequest } from '../generated/release_notes/UpdateTicketReleaseNoteRequest';

/** `409` `details.reason`: generating would overwrite a draft edited by hand. */
export const DRAFT_EDITED_REASON = 'draft_edited';
/** The section key of the tickets no template section takes. */
export const OTHER_SECTION_KEY = 'other';
