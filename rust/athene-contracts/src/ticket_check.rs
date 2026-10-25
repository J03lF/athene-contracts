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
}
