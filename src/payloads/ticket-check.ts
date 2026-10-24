/**
 * Ticket check wire types (intel layer R1): quality hints, the secret
 * scanner and "Ähnliche Tickets" while a ticket is written.
 *
 * Generated from `rust/athene-contracts/src/ticket_check.rs` by ts-rs.
 * The server sends no sentences: render `ticket_check.rule.<rule>.*`,
 * `ticket_check.reason.<code>` and `ticket_check.secret.<kind>`. A
 * switched-off feature answers 403 `ATH-FEAT-001` with
 * `FeatureDisabledDetails`.
 */
export type { AbstainReason } from '../generated/intel/AbstainReason';
export type { Abstention } from '../generated/intel/Abstention';
export type { CheckField } from '../generated/intel/CheckField';
export type { DecisionInput } from '../generated/intel/DecisionInput';
export type { DeleteIntelDataResponse } from '../generated/intel/DeleteIntelDataResponse';
export type { DuplicateCandidate } from '../generated/intel/DuplicateCandidate';
export type { FeatureDisabledDetails } from '../generated/intel/FeatureDisabledDetails';
export type { FixAction } from '../generated/intel/FixAction';
export type { HintSeverity } from '../generated/intel/HintSeverity';
export type { ModelInfo } from '../generated/intel/ModelInfo';
export type { QualityHint } from '../generated/intel/QualityHint';
export type { QualityReport } from '../generated/intel/QualityReport';
export type { QualityRule } from '../generated/intel/QualityRule';
export type { Reason } from '../generated/intel/Reason';
export type { ReasonCode } from '../generated/intel/ReasonCode';
export type { RecordDecisionsRequest } from '../generated/intel/RecordDecisionsRequest';
export type { RecordDecisionsResponse } from '../generated/intel/RecordDecisionsResponse';
export type { RelatedDoc } from '../generated/intel/RelatedDoc';
export type { ScoreBand } from '../generated/intel/ScoreBand';
export type { SecretFinding } from '../generated/intel/SecretFinding';
export type { SecretKind } from '../generated/intel/SecretKind';
export type { Strength } from '../generated/intel/Strength';
export type { SuggestionKind } from '../generated/intel/SuggestionKind';
export type { SwitchLevel } from '../generated/intel/SwitchLevel';
export type { TicketCheckFeature } from '../generated/intel/TicketCheckFeature';
export type { TicketCheckRequest } from '../generated/intel/TicketCheckRequest';
export type { TicketCheckResponse } from '../generated/intel/TicketCheckResponse';
export type { Verdict } from '../generated/intel/Verdict';
