//! Ticket check (intel layer R1, `artifacts/ticket-check/CONCEPT.md`):
//! while someone writes a ticket, local algorithms check it and explain
//! what they found. Quality rules, a secret scanner and the duplicate
//! radar ("Ähnliche Tickets"); no external service. Since Q020 the
//! quality score is a metascore over eight criteria, decided per criterion
//! by Athene's local quality model where it is sure, else by the rules.
//!
//! - `POST /work/workspaces/:workspace_id/ticket-check` checks a draft
//!   (action `ticket.check`; computes, writes nothing).
//! - `POST /work/workspaces/:workspace_id/intel/decisions` records what
//!   the person did with a suggestion (ignore, apply, …) as signed tokens.
//! - `POST /work/workspaces/:workspace_id/intel/data/delete` deletes what
//!   the workspace learned (`workspace.settings.edit`).
//! - `POST /work/workspaces/:workspace_id/ticket-check/compare` compares a
//!   saved ticket before and after an edit (`ticket.check`; computes,
//!   writes nothing).
//! - `GET /work/workspaces/:workspace_id/projects/:project_id/quality-review`
//!   reviews a page of the project's open tickets (anyone who can read the
//!   project; computes, writes nothing, notifies nobody).
//! - `POST …/projects/:project_id/quality-review/runs` reviews a page and
//!   remembers it as part of a run (ids, versions, scores and criterion
//!   states; no ticket text); `GET …/quality-review/last` answers the
//!   project's last finished run with the trend against the one before.
//! - `POST …/projects/:project_id/quality-review/requests` posts the
//!   confirmed "Reporter um Ergänzung bitten" comments (`comment.create`,
//!   at most 20 at once).
//!
//! Since the assistance (`athene-quality-assistance-v1`), a check also
//! carries the model package's writing assistance; see
//! [`WritingAssistance`].
//!
//! Every route answers `403 ATH-FEAT-001` with [`FeatureDisabledDetails`]
//! when a switch is off (platform flag `module.ticket_check`, the
//! workspace's `ticket_check_*` settings, the user's `ticket_check_*`
//! preferences).
//!
//! The server sends no sentences: the web renders `ticket_check.rule.<id>.*`,
//! `ticket_check.reason.<code>` and `ticket_check.secret.<kind>` from the
//! closed enums below. Request bodies refuse unknown fields.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use validator::Validate;

use crate::work::TicketPriority;

/// The platform flag of the whole function (`module.*`: on while no row
/// says otherwise).
pub const TICKET_CHECK_FLAG: &str = "module.ticket_check";

/// The most decisions one request records.
pub const MAX_DECISIONS: usize = 50;

// Keep wire names as token trees: utoipa must see the string literal in
// serde(rename), rather than an opaque forwarded literal fragment.
macro_rules! wire_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $( $(#[$vmeta:meta])* $variant:ident = $wire:tt ),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS)]
        #[ts(export_to = "intel/")]
        pub enum $name {
            $( $(#[$vmeta])* #[serde(rename = $wire)] $variant, )+
        }

        impl $name {
            /// Every value, in wire order.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// The wire name, the same string serde writes.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $( $name::$variant => $wire, )+
                }
            }

            /// The value with this wire name.
            pub fn parse(wire: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|v| v.as_str() == wire)
            }
        }
    };
}

wire_enum! {
    /// A part of the ticket check that can be switched off on its own.
    TicketCheckFeature {
        /// The whole function.
        All = "ticket_check",
        Duplicates = "duplicates",
        Quality = "quality",
        Secrets = "secrets",
    }
}

wire_enum! {
    /// Where a switch is off.
    SwitchLevel {
        Platform = "platform",
        Workspace = "workspace",
        User = "user",
    }
}

wire_enum! {
    /// The R1 quality rules (CONCEPT 2.3).
    QualityRule {
        TitleVague = "title.vague",
        TitleTooLong = "title.too_long",
        DescEmpty = "desc.empty",
        DescTemplateUnfilled = "desc.template_unfilled",
        BugStepsMissing = "bug.steps_missing",
        BugExpectedMissing = "bug.expected_missing",
        BugActualMissing = "bug.actual_missing",
        FeatureAcceptanceMissing = "feature.acceptance_missing",
        PriorityCriticalUnjustified = "priority.critical_unjustified",
        DuePast = "due.past",
    }
}

wire_enum! {
    /// How much a finding matters.
    HintSeverity {
        Info = "info",
        Warn = "warn",
        /// Must not be saved like this without a confirmation (secrets).
        Critical = "critical",
    }
}

wire_enum! {
    /// What the secret scanner looks for (CONCEPT 2.4).
    SecretKind {
        PrivateKey = "secret.private_key",
        CloudKey = "secret.cloud_key",
        VcsToken = "secret.vcs_token",
        ServiceToken = "secret.service_token",
        AtheneApiKey = "secret.athene_api_key",
        AtheneInvite = "secret.athene_invite",
        Jwt = "secret.jwt",
        AuthHeader = "secret.auth_header",
        UrlCredentials = "secret.url_credentials",
        PasswordAssignment = "secret.password_assignment",
        HighEntropy = "secret.high_entropy",
        Iban = "pii.iban",
        Card = "pii.card",
        TaxIdDe = "pii.tax_id_de",
        Email = "pii.email",
        Phone = "pii.phone",
    }
}

wire_enum! {
    /// One click that fixes a quality hint in the composer.
    FixAction {
        /// Append a "steps to reproduce" section.
        InsertSteps = "insert_steps",
        InsertExpected = "insert_expected",
        InsertActual = "insert_actual",
        InsertAcceptance = "insert_acceptance",
        /// Lower `critical` to `high`.
        LowerPriority = "lower_priority",
        ClearDueDate = "clear_due_date",
    }
}

wire_enum! {
    /// The field a finding is about.
    CheckField {
        Title = "title",
        Description = "description",
        Priority = "priority",
        DueDate = "due_date",
    }
}

wire_enum! {
    /// Why a duplicate candidate came up (CONCEPT 5.7).
    ReasonCode {
        TitleSimilar = "title_similar",
        TitleSpelling = "title_spelling",
        DescriptionOverlap = "description_overlap",
        Mentioned = "mentioned",
        SameProject = "same_project",
        SameType = "same_type",
        ExactError = "exact_error",
        SameComponent = "same_component",
        SameObservation = "same_observation",
        DifferentComponent = "different_component",
        DifferentSymptom = "different_symptom",
        Recent = "recent",
        Done = "done",
        Old = "old",
    }
}

wire_enum! {
    /// A reason's share of the score, in words.
    Strength {
        Strong = "strong",
        Medium = "medium",
        Weak = "weak",
    }
}

wire_enum! {
    /// The quality score as a band (CONCEPT 2.1).
    ScoreBand {
        /// ≥ 85
        Strong = "strong",
        /// 60–84
        Good = "good",
        /// 40–59
        Fair = "fair",
        /// < 40
        Thin = "thin",
    }
}

wire_enum! {
    /// What kind of suggestion a token or an abstention is about.
    SuggestionKind {
        Duplicate = "duplicate",
        Quality = "quality",
    }
}

wire_enum! {
    /// Why something was computed but not shown (CONCEPT 5.1). The UI does
    /// not show these; they count for the metrics.
    AbstainReason {
        /// Under the shown threshold or its lower confidence bound.
        BelowBound = "below_bound",
        /// The rule or kind runs in shadow mode.
        Shadow = "shadow",
        /// Over the attention budget (≤ 3 hints, ≤ 3 duplicates).
        Budget = "budget",
    }
}

wire_enum! {
    /// What the person did with a suggestion.
    Verdict {
        /// Applied the fix, or confirmed "same issue" / linked.
        Accepted = "accepted",
        /// "Not the same" for a duplicate.
        Rejected = "rejected",
        /// "Ignore" / "Passt so".
        Dismissed = "dismissed",
        /// Shown, saved without reacting.
        Ignored = "ignored",
        /// The hint went away because the text was fixed by hand.
        Fixed = "fixed",
    }
}

/// `POST /work/workspaces/:workspace_id/ticket-check`: a draft as the
/// composer has it. Nothing is stored.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "intel/")]
pub struct TicketCheckRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub project_id: Option<String>,
    /// The ticket type's key (`bug`, `feature`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    #[validate(length(max = 64))]
    pub ticket_type: Option<String>,
    #[validate(length(max = 500))]
    pub title: String,
    /// Markdown, as the editor holds it.
    #[serde(default)]
    #[validate(length(max = 65536))]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    #[schema(value_type = Option<String>)]
    pub priority: Option<TicketPriority>,
    /// `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub due_date: Option<String>,
    /// The person's calendar day (`YYYY-MM-DD`, their time zone); within a
    /// day of the server's, else the server's is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub today: Option<String>,
    /// Checking an existing ticket: it is no duplicate of itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ticket_id: Option<String>,
    /// A stable id of this composer session (the client's UUID): the same
    /// suggestion within one session is one vote, however the text moves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    #[validate(length(max = 64))]
    pub draft_id: Option<String>,
    /// The language of the writing assistance's questions (`de` when
    /// absent). Everything else stays keyed for the web's own texts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub locale: Option<AssistLocale>,
}

/// Versions behind an answer (CONCEPT 5.5): the same draft and versions
/// give the same answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct ModelInfo {
    pub version: u32,
    /// Rises when the workspace's learned state changes.
    pub revision: u32,
    pub pipeline_version: u16,
    /// RFC 3339, rounded down to the hour.
    pub as_of: String,
}

wire_enum! {
    /// Stable keys for the positive quality score ledger. The client
    /// translates these; no ticket text is stored in the explanation.
    QualityEvidenceCode {
        TitleContent = "title_content",
        TitleSpecificity = "title_specificity",
        TitleLength = "title_length",
        DescriptionSubstance = "description_substance",
        DescriptionStructure = "description_structure",
        BugSteps = "bug_steps",
        BugExpected = "bug_expected",
        BugActual = "bug_actual",
        BugEnvironment = "bug_environment",
        FeatureBenefit = "feature_benefit",
        FeatureAcceptance = "feature_acceptance",
        TaskOutcome = "task_outcome",
        SupportAffected = "support_affected",
        SupportAttempted = "support_attempted",
        ConcreteDetails = "concrete_details",
        PriorityUnjustified = "priority_unjustified",
        DuePast = "due_past",
    }
}

/// One scored dimension, including zero-point missing evidence. A small
/// contradiction has negative points and max_points = 0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct QualityEvidence {
    pub code: QualityEvidenceCode,
    pub points: i16,
    pub max_points: u8,
}

wire_enum! {
    /// The eight criteria of the quality metascore, in the model policy's
    /// binding order (`quality_policy.json`, `quality-v1`). The web renders
    /// `ticket_check.criterion.<id>.{title,feedback}`; the German feedback
    /// is the policy's `feedback_de`.
    QualityCriterion {
        /// Weight 3, every type.
        TitleInformative = "title_informative",
        /// Weight 4, every type.
        DescriptionInformative = "description_informative",
        /// Weight 3, every type.
        GoalClear = "goal_clear",
        /// Weight 2, every type.
        ContextSufficient = "context_sufficient",
        /// Weight 3, bugs only.
        ReproductionSufficient = "reproduction_sufficient",
        /// Weight 3, bugs only.
        ExpectedActual = "expected_actual",
        /// Weight 3, features and tasks.
        CompletionCriteria = "completion_criteria",
        /// Weight 1, only where the ticket depends on supporting material.
        EvidenceSufficient = "evidence_sufficient",
    }
}

wire_enum! {
    /// A criterion's verdict in the metascore.
    CriterionState {
        Pass = "pass",
        Fail = "fail",
        /// Neither the model (confidence < 0.7) nor a rule is sure: left
        /// out of the denominator and shown as "unsicher".
        Uncertain = "uncertain",
    }
}

wire_enum! {
    /// Who decided a criterion.
    CriterionSource {
        /// Athene's quality model, at a confidence of at least 0.7.
        Model = "model",
        /// The rule logic (the R1 evidence ledger).
        Rule = "rule",
        /// A formal rule: an empty title or description always fails.
        Formal = "formal",
    }
}

/// One applicable criterion of the metascore. Criteria that do not apply
/// to the ticket's type (or, for evidence, are not needed) are not listed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct CriterionResult {
    pub criterion: QualityCriterion,
    pub state: CriterionState,
    /// `None` while `uncertain`.
    pub source: Option<CriterionSource>,
    /// The policy weight.
    pub weight: u8,
    /// `weight` on pass, else 0.
    pub points: u8,
    /// The model's calibrated confidence in its top class, when the model
    /// ran (also when it was not sure and a rule decided).
    pub confidence: Option<f32>,
}

/// The quality metascore over the eight criteria (Q020): 100 × earned /
/// decided weights. Deviation from the model package: uncertain criteria
/// leave the denominator and are counted in `uncertain`, so there is a
/// score as long as one criterion is decided.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct MetaScore {
    /// 0–100, rounded half up; `None` when no criterion is decided.
    pub score: Option<u8>,
    pub band: Option<ScoreBand>,
    /// Weights of passed criteria.
    pub earned_weight: u8,
    /// Weights of decided (passed or failed) criteria: the denominator.
    pub assessed_weight: u8,
    /// Weights of every applicable criterion, decided or not.
    pub possible_weight: u8,
    /// How many applicable criteria are uncertain.
    pub uncertain: u8,
    /// Applicable criteria in policy order.
    pub criteria: Vec<CriterionResult>,
    /// Who could decide: `rules`, or `rules_model` when the model ran.
    pub engine: crate::athene_ai::AiQualityEngine,
    /// The model version that ran (`0.2.0-beta1`), else `None`.
    pub model_version: Option<String>,
    /// The model read only the first 256 tokens of the draft.
    pub input_truncated: bool,
}

/// One quality hint. Its text is `ticket_check.rule.<rule>.{title,why,fix}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct QualityHint {
    pub rule: QualityRule,
    pub severity: HintSeverity,
    pub field: CheckField,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fix: Option<FixAction>,
    /// For `intel/decisions`.
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct QualityReport {
    /// Positive evidence on a 0–100 scale; empty drafts have a floor of 10.
    pub score: u8,
    pub band: ScoreBand,
    /// Complete point ledger, independent of which hints are shown.
    #[serde(default)]
    #[schema(required = true)]
    pub evidence: Vec<QualityEvidence>,
    /// Shown hints, most important first (≤ 3).
    pub hints: Vec<QualityHint>,
    /// The metascore over the eight criteria; the web shows it instead of
    /// `score` when present. `None` from cores before Q020.
    #[serde(default)]
    pub metascore: Option<MetaScore>,
    /// The writing assistance of the model package (`assistance.py`):
    /// the model's confident failures as questions, ordered by weight.
    /// `None` without it; [`Self::assistance_unavailable`] says why.
    /// Absent from cores before the assistance.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub assistance: Option<WritingAssistance>,
    /// Why there is no assistance (`None` when there is one, and from
    /// cores before the assistance).
    #[serde(default)]
    #[ts(optional = nullable)]
    pub assistance_unavailable: Option<AssistanceUnavailable>,
}

/// A secret in the draft. Never the value: a masked preview and a
/// fingerprint that is neither stored nor logged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct SecretFinding {
    pub kind: SecretKind,
    pub severity: HintSeverity,
    pub field: CheckField,
    /// UTF-16 offsets in the field's text, `[start, end)`.
    pub start: u32,
    pub end: u32,
    /// `AKIA…YZ`; `••••` for short values.
    pub preview: String,
    /// First 16 hex digits of SHA-256 over kind and value: tells two
    /// findings apart without saying what they are.
    pub fingerprint: String,
}

/// One weighted reason ("Warum?", CONCEPT 5.7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct Reason {
    pub code: ReasonCode,
    /// Logit contribution, rounded to 0.1.
    pub weight: f32,
    pub strength: Strength,
    /// Words of the person's own draft that matched.
    pub terms: Vec<String>,
}

/// A ticket that may be the same issue ("Ähnliche Tickets").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct DuplicateCandidate {
    pub ticket_id: String,
    /// `WEB-12`.
    pub key: String,
    pub title: String,
    #[schema(value_type = String)]
    pub status: crate::work::TicketStatus,
    /// Finished (shown, but marked).
    pub done: bool,
    /// Calibrated probability that it is the same issue, 0–1.
    pub score: f32,
    /// Strongest first, at least one.
    pub reasons: Vec<Reason>,
    pub token: String,
}

/// A doc of the workspace about the same topic ("Passende Docs").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct RelatedDoc {
    pub doc_id: String,
    pub slug: String,
    /// Masked like every search result.
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct Abstention {
    pub kind: SuggestionKind,
    pub reason: AbstainReason,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct TicketCheckResponse {
    /// Identifies the draft's text (not reversible), for decisions.
    pub draft_hash: String,
    pub model: ModelInfo,
    /// `null` when quality is switched off.
    pub quality: Option<QualityReport>,
    pub secrets: Vec<SecretFinding>,
    pub duplicates: Vec<DuplicateCandidate>,
    /// Related docs, at most two; empty when duplicates are switched off.
    pub docs: Vec<RelatedDoc>,
    pub abstained: Vec<Abstention>,
}

/// One decision on a suggestion the server signed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "intel/")]
pub struct DecisionInput {
    #[validate(length(min = 1, max = 2048))]
    pub token: String,
    pub verdict: Verdict,
    /// The ticket the draft became, once saved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ticket_id: Option<String>,
}

/// `POST /work/workspaces/:workspace_id/intel/decisions`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "intel/")]
pub struct RecordDecisionsRequest {
    #[validate(length(min = 1, max = 50), nested)]
    pub decisions: Vec<DecisionInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct RecordDecisionsResponse {
    /// Decisions stored (a repeated vote replaces the earlier one).
    pub recorded: u32,
    /// Tokens refused: forged, expired or for another workspace.
    pub refused: u32,
}

/// `POST /work/workspaces/:workspace_id/intel/data/delete`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct DeleteIntelDataResponse {
    /// Rows removed over every `athene_intel_*` table.
    pub deleted: u32,
}

/// `details` of `403 ATH-FEAT-001`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct FeatureDisabledDetails {
    pub feature: TicketCheckFeature,
    pub level: SwitchLevel,
}

// ---------------------------------------------------------------------
// Writing assistance (the model package's `quality/assistance.py`,
// contract `athene-quality-assistance-v1`): fixed derivations of the eight
// criteria, no generated text. The questions, sections and notice are the
// package's own texts in German or English, sent as they are (the one
// place where the core sends sentences: they belong to the model package
// and its goldens, not to the web).
// ---------------------------------------------------------------------

/// The only assistance contract this adapter speaks.
pub const ASSISTANCE_CONTRACT: &str = "athene-quality-assistance-v1";
/// Tickets one backlog review request evaluates at most.
pub const MAX_REVIEW_PAGE: u32 = 50;
/// Tickets one backlog review run covers at most; the web pages on.
pub const MAX_REVIEW_RUN: u32 = 200;

wire_enum! {
    /// The language of the assistance's texts.
    AssistLocale {
        De = "de",
        En = "en",
    }
}

wire_enum! {
    /// What the assistance found for one ticket.
    AssistanceState {
        /// At least one confident failure: there are questions.
        NeedsDetails = "needs_details",
        /// No confident failure, but some criteria are uncertain.
        Uncertain = "uncertain",
        /// The text exceeds the evaluation window: no score, no list.
        Partial = "partial",
        /// "Keine Mängel erkannt": no defect found, which is no approval.
        NoFlags = "no_flags",
    }
}

wire_enum! {
    /// How much of the text the evaluation read.
    AssistanceScope {
        Complete = "complete",
        Partial = "partial",
    }
}

wire_enum! {
    /// A criterion's state in the model's own evaluation (the package's
    /// `scoring.score`, before Athene combines it with the rules).
    ModelCriterionState {
        Pass = "pass",
        Fail = "fail",
        Uncertain = "uncertain",
        NotApplicable = "not_applicable",
    }
}

wire_enum! {
    /// Why two evaluations are not compared (`reason`), or why their
    /// scores are not (`score_delta_unavailable_reason`, which adds the
    /// last two).
    ComparisonReason {
        /// Model, model version, contract or policy differ.
        EvaluationVersionChanged = "evaluation_version_changed",
        TicketIdentityChanged = "ticket_identity_changed",
        TicketTypeChanged = "ticket_type_changed",
        /// Either text exceeds the evaluation window.
        PartialInput = "partial_input",
        /// A score is unknown because a criterion is uncertain.
        UncertainScore = "uncertain_score",
        /// Applicability or weights differ: another denominator.
        DifferentApplicabilityOrWeights = "different_applicability_or_weights",
    }
}

wire_enum! {
    /// Why a check or a review has no model assistance.
    AssistanceUnavailable {
        /// The workspace chose "Nur Regeln".
        RulesOnly = "rules_only",
        /// No model is active (off, not installed, being checked).
        NoModel = "no_model",
        /// The active model has no `athene-quality-assistance-v1`.
        ModelWithoutAssistance = "model_without_assistance",
        /// The ticket's type is none of the model's four.
        TypeNotSupported = "type_not_supported",
        /// The inference failed or timed out.
        Failed = "failed",
    }
}

wire_enum! {
    /// Who evaluated a backlog review row.
    ReviewEngine {
        /// The model and its assistance contract.
        Assistance = "assistance",
        /// The rule logic alone (the workspace chose "Nur Regeln", no
        /// model with assistance is active, or the type is not one of the
        /// model's four).
        Rules = "rules",
    }
}

/// One question or request for a criterion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct AssistImprovement {
    pub criterion_id: QualityCriterion,
    /// The part of the ticket it is about ("Ist und Soll").
    pub section: String,
    /// The concrete question or request.
    pub prompt: String,
    /// The policy weight; the list is ordered by it.
    pub weight: u8,
    /// The model's calibrated confidence in the failure; `None` for a
    /// rule-based backlog row.
    pub confidence: Option<f64>,
}

/// `writing_assistance` for one draft, field for field as the package
/// answers it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct WritingAssistance {
    /// `athene-quality-assistance-v1`.
    pub contract_version: String,
    /// SHA-256 (hex) of the UTF-8 model input text after NFC and secret
    /// masking, before tokenisation. Binds the answer to the text it is
    /// about; no security feature.
    pub input_fingerprint: String,
    pub model_version: String,
    pub locale: AssistLocale,
    pub state: AssistanceState,
    pub scope: AssistanceScope,
    /// The package's score (two decimals): `None` when partial or when a
    /// criterion is uncertain.
    pub displayed_score: Option<f64>,
    /// Lowest and highest possible score; `None` when partial.
    pub displayed_score_range: Option<[Option<f64>; 2]>,
    /// Confident failures, heaviest first; empty when partial.
    pub improvements: Vec<AssistImprovement>,
    /// Criteria the model is not sure about, in policy order.
    pub uncertain_criteria: Vec<QualityCriterion>,
    /// Why there is no score and no list (`partial` only).
    pub notice: Option<String>,
}

/// One side of an edit: what the model reads of the ticket.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "intel/")]
pub struct EditSide {
    /// The ticket type's key.
    #[validate(length(max = 64))]
    pub ticket_type: String,
    #[validate(length(max = 500))]
    pub title: String,
    #[serde(default)]
    #[validate(length(max = 65536))]
    pub description: String,
}

/// `POST /work/workspaces/:workspace_id/ticket-check/compare`: a saved
/// ticket before and after an edit (the web keeps the version it had
/// loaded). Computes, stores nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "intel/")]
pub struct CompareEditsRequest {
    #[validate(length(min = 1, max = 64))]
    pub ticket_id: String,
    #[validate(nested)]
    pub before: EditSide,
    #[validate(nested)]
    pub after: EditSide,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub locale: Option<AssistLocale>,
}

/// A criterion whose state changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct CriterionChange {
    pub criterion_id: QualityCriterion,
    pub before: ModelCriterionState,
    pub after: ModelCriterionState,
}

/// `compare_edits`, field for field as the package answers it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct EditComparison {
    pub contract_version: String,
    /// `false`: model, policy, type or ticket changed, or a text is
    /// partial; then every list is empty.
    pub comparable: bool,
    pub reason: Option<ComparisonReason>,
    pub before_fingerprint: String,
    pub after_fingerprint: String,
    /// fail → pass only.
    pub resolved_criteria: Vec<QualityCriterion>,
    /// Newly failed (from pass, uncertain or not applicable).
    pub new_flags: Vec<QualityCriterion>,
    /// Now not applicable; not counted as resolved.
    pub no_longer_applicable: Vec<QualityCriterion>,
    pub changed_criteria: Vec<CriterionChange>,
    /// After minus before, two decimals; `None` unless both scores are
    /// certain with the same applicability and weights.
    pub score_delta: Option<f64>,
    pub score_delta_unavailable_reason: Option<ComparisonReason>,
    pub before: WritingAssistance,
    pub after: WritingAssistance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct CompareEditsResponse {
    /// `None` when there is no model assistance; `unavailable` says why.
    pub comparison: Option<EditComparison>,
    pub unavailable: Option<AssistanceUnavailable>,
}

/// `GET /work/workspaces/:workspace_id/projects/:project_id/quality-review`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "intel/")]
pub struct BacklogReviewQuery {
    /// Position in the project's open tickets (oldest first), default 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub offset: Option<u32>,
    /// 1..=50 (default 25).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub locale: Option<AssistLocale>,
}

/// One reviewed ticket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct BacklogReviewItem {
    pub ticket_id: String,
    /// `WEB-12`.
    pub key: String,
    /// The title as the backlog shows it.
    pub title: String,
    pub ticket_type: String,
    /// 1-based position in the project's open tickets (the tie-break).
    pub source_row: u32,
    pub engine: ReviewEngine,
    /// The weights of `improvements` together.
    pub flagged_weight: u32,
    pub state: AssistanceState,
    /// `engine = assistance`: the package's score (two decimals, `None`
    /// when uncertain or partial). `rules`: the rules' metascore.
    pub displayed_score: Option<f64>,
    pub displayed_score_range: Option<[Option<f64>; 2]>,
    pub improvements: Vec<AssistImprovement>,
    pub uncertain_criteria: Vec<QualityCriterion>,
    pub notice: Option<String>,
    /// `engine = assistance` only.
    pub input_fingerprint: Option<String>,
    /// The applicable criteria in policy order (pass ✓, fail ✗, uncertain
    /// ?), with their weights: the row's checklist. Empty when partial.
    #[serde(default)]
    #[schema(required = true)]
    pub criteria: Vec<ReviewCriterion>,
    /// The ticket's `version` when it was reviewed (3.7).
    #[serde(default)]
    #[schema(required = true)]
    pub ticket_version: i64,
    #[serde(default)]
    pub assignee_id: Option<String>,
    #[serde(default)]
    #[schema(required = true)]
    pub reporter_id: String,
    #[serde(default)]
    pub sprint_id: Option<String>,
    /// Stored results only: the ticket was written since it was reviewed
    /// ("seitdem geändert"); the row is still the old evaluation.
    #[serde(default)]
    #[schema(required = true)]
    pub changed_since: bool,
    /// Stored results only: when someone asked the reporter for the
    /// missing details from this review ("Nachgefragt"), RFC 3339.
    #[serde(default)]
    pub asked_at: Option<String>,
    /// Stored results with a comparable previous run only: how the row
    /// moved since then.
    #[serde(default)]
    pub trend: Option<ReviewTrendMark>,
}

/// One applicable criterion of a review row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct ReviewCriterion {
    pub criterion: QualityCriterion,
    /// `pass`, `fail` or `uncertain` (not applicable ones are left out).
    pub state: ModelCriterionState,
    /// The policy weight; a pass earns it.
    pub weight: u8,
}

wire_enum! {
    /// How a ticket's review moved against the previous run (same model).
    ReviewTrendMark {
        Better = "better",
        Worse = "worse",
        Same = "same",
        /// Not in the previous run.
        New = "new",
    }
}

wire_enum! {
    /// Why a stored review has no comparison with the run before it.
    ReviewTrendUnavailable {
        /// There is no earlier run of this project.
        NoPrevious = "no_previous",
        /// The earlier run used another engine, model or policy version.
        DifferentModel = "different_model",
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct BacklogReviewCounts {
    pub needs_details: u32,
    pub uncertain: u32,
    pub partial: u32,
    pub no_flags: u32,
}

/// The rows by state. `needs_details` is ordered by the flagged weight,
/// heaviest first, then by position: quality feedback only, not business
/// urgency or ticket priority. The others keep the position order.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct BacklogReviewGroups {
    pub needs_details: Vec<BacklogReviewItem>,
    pub uncertain: Vec<BacklogReviewItem>,
    pub partial: Vec<BacklogReviewItem>,
    pub no_flags: Vec<BacklogReviewItem>,
}

/// One page of a backlog review: read-only, nothing is written back and
/// nobody is notified.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct BacklogReviewPage {
    /// Who evaluated the page: `assistance` (model rows, plus rule rows
    /// for types the model does not read) or `rules` throughout.
    pub engine: ReviewEngine,
    /// Why the page is rules only (`None` with the model).
    pub unavailable: Option<AssistanceUnavailable>,
    /// `athene-quality-assistance-v1` with the model, else `None`.
    pub contract_version: Option<String>,
    pub model_version: Option<String>,
    pub policy_version: Option<String>,
    /// Open tickets of the project the viewer can see.
    pub total_open: u32,
    pub offset: u32,
    pub limit: u32,
    /// Rows on this page.
    pub tickets: u32,
    pub counts: BacklogReviewCounts,
    /// "quality feedback weight only; not business urgency or ticket
    /// priority".
    pub ordering: String,
    pub groups: BacklogReviewGroups,
    /// `POST …/quality-review/runs` only: the stored run this page was
    /// added to (send it with the next page). `None` from the read-only
    /// `GET`, or when the page could not be stored.
    #[serde(default)]
    pub run_id: Option<String>,
}

/// `POST /work/workspaces/:workspace_id/projects/:project_id/quality-review/runs`:
/// one page of a backlog review that is remembered. The first page
/// (without `run_id`) starts a run; the next ones name it. A run is the
/// project's last result once its last page (`MAX_REVIEW_RUN` tickets or
/// the end of the open tickets) arrived. Stored per ticket: ids, version,
/// score, group and criterion states; no ticket text.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "intel/")]
pub struct BacklogReviewRunRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub offset: Option<u32>,
    /// 1..=50 (default 25).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub locale: Option<AssistLocale>,
    /// The run of the earlier pages; absent on the first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    #[validate(length(min = 36, max = 36))]
    pub run_id: Option<String>,
}

/// `GET …/quality-review/last?locale`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "intel/")]
pub struct BacklogReviewLastQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub locale: Option<AssistLocale>,
}

/// Who ran a stored review, when, with what.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct StoredReviewHead {
    pub run_id: String,
    /// When the last page arrived, RFC 3339.
    pub reviewed_at: String,
    pub reviewed_by: String,
    pub engine: ReviewEngine,
    pub model_version: Option<String>,
    pub policy_version: Option<String>,
    pub contract_version: Option<String>,
    /// Position of the run's first ticket in the open tickets (0, 200, …).
    pub start: u32,
    /// Tickets the run reviewed.
    pub tickets: u32,
    /// Open tickets of the project when it ran.
    pub total_open: u32,
}

/// The comparison with the run before (same engine, model and policy).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct BacklogReviewTrend {
    pub better: u32,
    pub worse: u32,
    pub same: u32,
    /// Reviewed now, not then.
    pub new: u32,
    /// When the earlier run finished, RFC 3339.
    pub previous_at: String,
}

/// The project's last remembered review, rebuilt for today's tickets:
/// titles and keys are read now (no text is stored), the questions come
/// from the package in `locale`, deleted or closed tickets are left out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct BacklogReviewLast {
    /// `None`: the project was never reviewed (or its run is unfinished).
    pub review: Option<StoredReviewHead>,
    pub counts: BacklogReviewCounts,
    pub ordering: String,
    pub groups: BacklogReviewGroups,
    pub trend: Option<BacklogReviewTrend>,
    pub trend_unavailable: Option<ReviewTrendUnavailable>,
}

/// The most comments one "Reporter um Ergänzung bitten" posts.
pub const MAX_ASK_REPORTER: usize = 20;

/// One comment of "Reporter um Ergänzung bitten".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "intel/")]
pub struct AskReporterItem {
    #[validate(length(min = 36, max = 36))]
    pub ticket_id: String,
    /// The text the person confirmed (Markdown, mentions as
    /// `[@Name](mention:<id>)`). The server puts the label line
    /// "Athene AI · Vorschlag" above it.
    #[validate(length(min = 1, max = 4000))]
    pub body: String,
}

/// `POST /work/workspaces/:workspace_id/projects/:project_id/quality-review/requests`:
/// posts each confirmed comment as the current person (`comment.create`
/// on the ticket), at most [`MAX_ASK_REPORTER`] at once; never sent
/// without the person's confirmation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "intel/")]
pub struct AskReporterRequest {
    #[validate(length(min = 1, max = 20), nested)]
    pub items: Vec<AskReporterItem>,
    /// The label line's language (`de` when absent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub locale: Option<AssistLocale>,
}

wire_enum! {
    /// What happened to one comment.
    AskReporterOutcome {
        Sent = "sent",
        /// No `comment.create` on that ticket.
        Forbidden = "forbidden",
        /// Not a ticket of this project (or gone).
        NotFound = "not_found",
        /// Listed twice in the request: sent once.
        Duplicate = "duplicate",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct AskReporterResult {
    pub ticket_id: String,
    pub outcome: AskReporterOutcome,
    /// `sent`: the new comment.
    pub comment_id: Option<String>,
    /// `sent`: "Nachgefragt" since, RFC 3339.
    pub asked_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "intel/")]
pub struct AskReporterResponse {
    pub results: Vec<AskReporterResult>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn wire_names_round_trip_through_serde() {
        for rule in QualityRule::ALL {
            let json = serde_json::to_string(rule).unwrap();
            assert_eq!(json, format!("\"{}\"", rule.as_str()));
            assert_eq!(QualityRule::parse(rule.as_str()), Some(*rule));
        }
        for kind in SecretKind::ALL {
            let back: SecretKind = serde_json::from_str(&format!("\"{}\"", kind.as_str())).unwrap();
            assert_eq!(back, *kind);
        }
        assert_eq!(QualityRule::ALL.len(), 10);
        assert_eq!(SecretKind::ALL.len(), 16);
    }

    #[test]
    fn every_wire_enum_schema_matches_serialized_values() {
        fn assert_schema<T: Serialize + utoipa::PartialSchema>(values: &[T]) {
            let schema = serde_json::to_value(T::schema()).unwrap();
            let serialized = serde_json::to_value(values).unwrap();
            assert_eq!(schema["type"], "string", "{}", std::any::type_name::<T>());
            assert_eq!(schema["enum"], serialized, "{}", std::any::type_name::<T>());
        }

        assert_schema(TicketCheckFeature::ALL);
        assert_schema(SwitchLevel::ALL);
        assert_schema(QualityRule::ALL);
        assert_schema(HintSeverity::ALL);
        assert_schema(SecretKind::ALL);
        assert_schema(FixAction::ALL);
        assert_schema(CheckField::ALL);
        assert_schema(ReasonCode::ALL);
        assert_schema(Strength::ALL);
        assert_schema(ScoreBand::ALL);
        assert_schema(SuggestionKind::ALL);
        assert_schema(AbstainReason::ALL);
        assert_schema(Verdict::ALL);
        assert_schema(QualityEvidenceCode::ALL);
        assert_schema(QualityCriterion::ALL);
        assert_schema(CriterionState::ALL);
        assert_schema(CriterionSource::ALL);
        assert_schema(AssistLocale::ALL);
        assert_schema(AssistanceState::ALL);
        assert_schema(AssistanceScope::ALL);
        assert_schema(ModelCriterionState::ALL);
        assert_schema(ComparisonReason::ALL);
        assert_schema(AssistanceUnavailable::ALL);
        assert_schema(ReviewEngine::ALL);
    }

    #[test]
    fn criteria_keep_the_policy_order() {
        let ids: Vec<&str> = QualityCriterion::ALL.iter().map(|c| c.as_str()).collect();
        assert_eq!(
            ids,
            [
                "title_informative",
                "description_informative",
                "goal_clear",
                "context_sufficient",
                "reproduction_sufficient",
                "expected_actual",
                "completion_criteria",
                "evidence_sufficient",
            ]
        );
    }

    #[test]
    fn evidence_is_required_in_output_schema_but_defaults_for_older_input() {
        use utoipa::PartialSchema;

        let report: QualityReport = serde_json::from_value(serde_json::json!({
            "score": 10,
            "band": "thin",
            "hints": [],
        }))
        .unwrap();
        assert!(report.evidence.is_empty());
        assert!(report.metascore.is_none());
        let serialized = serde_json::to_value(report).unwrap();
        assert_eq!(serialized["evidence"], serde_json::json!([]));

        let schema = serde_json::to_value(QualityReport::schema()).unwrap();
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("evidence")));
    }

    #[test]
    fn requests_refuse_unknown_fields_and_bound_their_size() {
        assert!(serde_json::from_str::<TicketCheckRequest>(r#"{"title":"x","nope":1}"#).is_err());
        let ok: TicketCheckRequest = serde_json::from_str(r#"{"title":"Login fails"}"#).unwrap();
        assert!(ok.validate().is_ok());
        let long = TicketCheckRequest {
            title: "x".repeat(501),
            ..TicketCheckRequest::default()
        };
        assert!(long.validate().is_err());
        let none = RecordDecisionsRequest { decisions: vec![] };
        assert!(none.validate().is_err());
    }
    #[test]
    fn assistance_keeps_the_package_shape() {
        let wire = serde_json::json!({
            "contract_version": "athene-quality-assistance-v1",
            "input_fingerprint": "2fb05a6bf1417b283b79d000da746273968aab8a15f4ebfa008ecb412b929608",
            "model_version": "0.5.0-beta4", "locale": "de", "state": "needs_details",
            "scope": "complete", "displayed_score": 66.67, "displayed_score_range": [66.67, 66.67],
            "improvements": [{"criterion_id": "goal_clear", "section": "Ziel",
                "prompt": "Beschreibe ein eindeutiges gewünschtes Ergebnis; kläre widersprüchliche Anforderungen.",
                "weight": 3, "confidence": 0.8217225121019534}],
            "uncertain_criteria": [], "notice": null
        });
        let back: WritingAssistance = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(back.state, AssistanceState::NeedsDetails);
        assert_eq!(serde_json::to_value(&back).unwrap(), wire);
        let partial: WritingAssistance = serde_json::from_value(serde_json::json!({
            "contract_version": ASSISTANCE_CONTRACT, "input_fingerprint": "x",
            "model_version": "v", "locale": "en", "state": "partial", "scope": "partial",
            "displayed_score": null, "displayed_score_range": null, "improvements": [],
            "uncertain_criteria": ["goal_clear"], "notice": "n"
        }))
        .unwrap();
        assert!(partial.displayed_score_range.is_none());
    }

    #[test]
    fn compare_requests_validate_both_sides() {
        let ok: CompareEditsRequest = serde_json::from_value(serde_json::json!({
            "ticket_id": "t", "before": {"ticket_type": "task", "title": "a", "description": ""},
            "after": {"ticket_type": "task", "title": "b"}, "locale": "en"
        }))
        .unwrap();
        assert!(ok.validate().is_ok());
        let long = CompareEditsRequest {
            after: EditSide {
                title: "x".repeat(501),
                ..EditSide::default()
            },
            ..ok.clone()
        };
        assert!(long.validate().is_err());
        assert!(
            serde_json::from_value::<CompareEditsRequest>(serde_json::json!({
                "ticket_id": "t", "before": {"ticket_type": "task", "title": "a", "x": 1},
                "after": {"ticket_type": "task", "title": "b"}
            }))
            .is_err()
        );
        let older: QualityReport = serde_json::from_value(serde_json::json!({
            "score": 10, "band": "thin", "hints": [], "metascore": null
        }))
        .unwrap();
        assert!(older.assistance.is_none() && older.assistance_unavailable.is_none());
    }

    #[test]
    fn asking_reporters_is_bounded_and_older_review_rows_still_read() {
        let id = "0b9f3f3e-8f0e-4a43-9a52-1c0d6e1d2a10";
        let item = AskReporterItem {
            ticket_id: id.into(),
            body: "Kannst du das ergänzen?".into(),
        };
        let ok = AskReporterRequest {
            items: vec![item.clone(); MAX_ASK_REPORTER],
            locale: None,
        };
        assert!(ok.validate().is_ok());
        let many = AskReporterRequest {
            items: vec![item.clone(); MAX_ASK_REPORTER + 1],
            locale: None,
        };
        assert!(many.validate().is_err(), "at most 20 per action");
        let empty_body = AskReporterRequest {
            items: vec![AskReporterItem {
                body: String::new(),
                ..item
            }],
            locale: None,
        };
        assert!(empty_body.validate().is_err());
        assert!(serde_json::from_value::<BacklogReviewRunRequest>(
            serde_json::json!({ "offset": 0, "run_id": id, "x": 1 })
        )
        .is_err());
        let short = BacklogReviewRunRequest {
            run_id: Some("nope".into()),
            ..BacklogReviewRunRequest::default()
        };
        assert!(short.validate().is_err());
        // A row from before review2 (no checklist, no version) still reads.
        let older: BacklogReviewItem = serde_json::from_value(serde_json::json!({
            "ticket_id": id, "key": "WEB-1", "title": "t", "ticket_type": "bug",
            "source_row": 1, "engine": "rules", "flagged_weight": 0, "state": "no_flags",
            "displayed_score": null, "displayed_score_range": null, "improvements": [],
            "uncertain_criteria": [], "notice": null, "input_fingerprint": null
        }))
        .unwrap();
        assert!(older.criteria.is_empty() && !older.changed_since && older.trend.is_none());
    }
}
