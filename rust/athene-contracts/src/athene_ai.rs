//! Athenos AI: one brand and one place to control every intelligence
//! function of Athene. In R1 that is the ticket check (quality hints,
//! secret protection, "Ähnliche Tickets", local algorithms); Q020 adds
//! Athenos' own local quality model ("Athenos AI Beta 0.2.0").
//!
//! Three levels decide whether it runs, the first one here:
//!
//! - **Platform** (admin console, [`AiVisibility`]): `off`, `preview`
//!   (platform admins and enrolled workspaces only) or `on`. The feature
//!   flag `module.ticket_check` stays the hard kill switch: off there is
//!   `off` here, whatever is stored.
//! - **Workspace**: the `ticket_check_*` workspace settings.
//! - **Person**: the `ticket_check*` preferences.
//!
//! Routes:
//!
//! - `GET /work/workspaces/:workspace_id/capabilities` carries
//!   [`AtheneAiAvailability`] (`athene_ai`), so the web hides everything
//!   the platform does not allow.
//! - `GET /work/workspaces/:workspace_id/athene-ai` — the workspace's page
//!   (availability, beta enrollment, this week's numbers). Members.
//! - `POST` / `DELETE /work/workspaces/:workspace_id/athene-ai/beta` —
//!   apply for the beta, withdraw the request (`workspace.settings.edit`,
//!   only in `preview`).
//! - `GET` / `PUT /admin/athene-ai` — the platform control.
//! - `POST /admin/athene-ai/workspaces`, `DELETE
//!   /admin/athene-ai/workspaces/:workspace_id` — enrol a workspace
//!   directly, remove one.
//! - `POST /admin/athene-ai/requests/:workspace_id/approve|reject`.
//! - `GET /admin/athene-ai/model/licenses?version=` — the licence files
//!   shipped with an installed version of the quality model (the active
//!   one without `version`).
//!
//! The quality model (models step): exactly one version is loaded at any
//! time, platform-wide. The platform admin turns it on or off and picks the
//! active version among the installed ones ([`AiModelStatus::versions`]);
//! a new pick is checked in the background and swapped in only once it
//! passed, the old session is dropped right after. Workspaces never pick a
//! version: their owners and admins choose "Qualitätsbewertung" (the
//! workspace setting `ticket_check_quality_engine`, [`AiQualityEngine`]).
//!
//! The admin routes answer the whole [`AtheneAiAdminResponse`] after a
//! change. A switched-off platform answers the workspace routes with
//! `403 ATH-FEAT-001` (level `platform`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

macro_rules! ai_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $( $(#[$vmeta:meta])* $variant:ident = $wire:literal ),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ToSchema, TS)]
        #[ts(export_to = "ai/")]
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

ai_enum! {
    /// Who sees Athenos AI on this instance.
    ///
    /// There is no `dev_only`: modules do not learn the host's
    /// `FENRIR_ENV` (the host strips every `FENRIR_*` variable from a
    /// module's environment and passes none back), so the core cannot tell
    /// a test instance from production.
    #[derive(Default)]
    AiVisibility {
        /// Hidden everywhere; the API answers as switched off.
        Off = "off",
        /// Platform admins, and workspaces enrolled in the beta.
        Preview = "preview",
        /// Every workspace; each can still switch it off.
        #[default]
        On = "on",
    }
}

ai_enum! {
    /// A workspace's place in the beta.
    #[derive(Default)]
    AiEnrollmentStatus {
        /// Never applied, or the request was withdrawn or removed.
        #[default]
        None = "none",
        /// Applied; waits for a platform admin.
        Requested = "requested",
        /// In the beta (approved, or added by a platform admin).
        Enrolled = "enrolled",
        /// The request was turned down; the workspace may apply again.
        Rejected = "rejected",
    }
}

ai_enum! {
    /// What answers the quality check right now.
    AiEngine {
        /// The R1 algorithms only.
        AlgorithmsR1 = "algorithms_r1",
        /// The R1 algorithms plus Athenos' own quality model (Q020).
        AlgorithmsModel = "algorithms_model",
    }
}

ai_enum! {
    /// Athenos' own quality model, as the core found it.
    AiModelState {
        /// No package at the configured place (or no place configured).
        NotInstalled = "not_installed",
        /// The package is there and waits for its check.
        Installed = "installed",
        /// Checksums, session load and the golden cases are running.
        Checking = "checking",
        /// Checked: every golden case matched; the model may answer.
        Ready = "ready",
        /// The package failed a check; the rules carry on alone.
        Error = "error",
    }
}

ai_enum! {
    /// How a workspace's quality score is decided (Settings → Athenos AI,
    /// "Qualitätsbewertung"; the workspace setting
    /// `ticket_check_quality_engine`, unset = `rules_model`).
    #[derive(Default)]
    AiQualityEngine {
        /// The rule logic alone.
        Rules = "rules",
        /// Per criterion, whoever is sure decides: the model at a
        /// confidence of at least 0.7, the rules otherwise. In effect only
        /// while the model is `ready`; until then the rules answer alone.
        #[default]
        RulesModel = "rules_model",
    }
}

ai_enum! {
    /// Why the model is not ready. The web words it; `detail` names the
    /// file or golden case when there is one.
    AiModelError {
        /// Turned off by configuration (`ATHENE_QUALITY_MODEL=off`).
        Disabled = "disabled",
        /// No models directory: `ATHENE_MODELS_DIR` unset and no home.
        NoModelsDir = "no_models_dir",
        /// `athene_model.json` or `golden.jsonl` is missing.
        NotFound = "not_found",
        /// A contract other than `athene-quality-onnx-v1`.
        ContractUnsupported = "contract_unsupported",
        /// The metadata or policy does not match what this core reads.
        InvalidPackage = "invalid_package",
        /// A file's SHA-256 differs from `athene_model.json`.
        ChecksumMismatch = "checksum_mismatch",
        /// The tokenizer or the ONNX graph could not be loaded.
        LoadFailed = "load_failed",
        /// A golden case gave other token ids, confidences or decisions.
        GoldenMismatch = "golden_mismatch",
    }
}

ai_enum! {
    /// Who decides which quality model runs.
    #[derive(Default)]
    AiModelControl {
        /// The platform admin (Admin → Athenos AI, "Athenos-Modell").
        #[default]
        Admin = "admin",
        /// `ATHENE_QUALITY_MODEL=off`, the hard kill switch: never loaded,
        /// whatever is stored.
        EnvOff = "env_off",
    }
}

/// `athene_ai` of the capabilities: whether Athenos AI is there for the
/// caller in this workspace, as far as the platform goes. The workspace's
/// and the person's own switches come on top (`ticket_check_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AtheneAiAvailability {
    /// Runs here for the caller: `on`, or `preview` and (enrolled or a
    /// platform admin).
    pub available: bool,
    /// The platform's mode in effect (`off` while the kill switch
    /// `module.ticket_check` is off).
    pub visibility: AiVisibility,
    /// The workspace's beta enrollment.
    pub enrollment: AiEnrollmentStatus,
    /// The caller may apply for the beta here or withdraw the request:
    /// `preview`, an owner or admin of the workspace, not enrolled yet.
    pub can_enroll: bool,
}

impl AtheneAiAvailability {
    /// Nothing there: what [`crate::policy::capabilities`] answers until
    /// the core fills in the platform's verdict.
    pub const HIDDEN: Self = Self {
        available: false,
        visibility: AiVisibility::Off,
        enrollment: AiEnrollmentStatus::None,
        can_enroll: false,
    };
}

/// One number pair of this week's Athenos AI on a workspace page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AiFeatureStat {
    /// Suggestions somebody acted on (applied, dismissed, rejected, …).
    pub rated: u32,
    /// Of those, applied or confirmed (`accepted`, `fixed`).
    pub accepted: u32,
}

/// The last seven days, from the intel decisions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AiWeekStats {
    /// Start of the window (RFC 3339).
    pub since: String,
    pub duplicates: AiFeatureStat,
    pub quality: AiFeatureStat,
}

/// A workspace's beta enrollment as its owners see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AiEnrollment {
    pub status: AiEnrollmentStatus,
    /// RFC 3339.
    pub requested_at: Option<String>,
    /// Display name of who applied.
    pub requested_by_name: Option<String>,
    /// When a platform admin approved or rejected it (RFC 3339).
    pub decided_at: Option<String>,
}

/// `GET /work/workspaces/:workspace_id/athene-ai` (and the beta routes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AtheneAiWorkspaceResponse {
    pub availability: AtheneAiAvailability,
    /// `None` until the workspace first applies or is added.
    pub enrollment: Option<AiEnrollment>,
    /// This week's numbers; `None` while Athenos AI is not available here.
    pub stats: Option<AiWeekStats>,
    /// The platform's quality model, while one is active; `None`: the
    /// workspace's quality check runs on the rules alone.
    pub model: Option<AiWorkspaceModel>,
}

/// A workspace in the beta list or the request queue of the console.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AiBetaWorkspace {
    pub workspace_id: String,
    pub workspace_name: String,
    pub status: AiEnrollmentStatus,
    pub requested_by_name: Option<String>,
    /// RFC 3339.
    pub requested_at: Option<String>,
    /// RFC 3339.
    pub decided_at: Option<String>,
}

/// A workspace for the console's picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AiWorkspaceRef {
    pub id: String,
    pub name: String,
}

/// Athenos' own quality model: which package, its state and how fast it
/// answers on this machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AiQualityModelStatus {
    /// `athene-quality`.
    pub id: String,
    /// `0.2.0-beta1`: the configured version (a path below the models
    /// directory), also before the package was read.
    pub version: String,
    /// The package's contract once read (`athene-quality-onnx-v1`).
    pub contract_version: Option<String>,
    pub state: AiModelState,
    /// Set in `error` and `not_installed`.
    pub error: Option<AiModelError>,
    /// The file or golden case the error is about; never a secret.
    pub error_detail: Option<String>,
    pub goldens_passed: u32,
    pub goldens_total: u32,
    /// Largest confidence difference over the golden cases.
    pub max_confidence_diff: Option<f64>,
    /// Inference (tokenizer and model) over recent checks, else over the
    /// golden cases; milliseconds.
    pub latency_p50_ms: Option<f32>,
    pub latency_p95_ms: Option<f32>,
    pub latency_samples: u32,
    /// Checksums, session load and goldens, milliseconds.
    pub check_ms: Option<u32>,
    /// RFC 3339, when the check finished.
    pub checked_at: Option<String>,
    /// `LICENSES.md` and `BASE_LICENSE.txt` ship with the package
    /// (`GET /admin/athene-ai/model/licenses`).
    pub licenses: bool,
    /// The package's own `training_source` (`assistant_authored_compositional`).
    pub training_source: Option<String>,
}

/// The last check of one installed version (since this core started).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AiModelCheck {
    /// Checksums, session load and every golden case passed.
    pub ok: bool,
    pub error: Option<AiModelError>,
    /// The file or golden case the error is about; never a secret.
    pub error_detail: Option<String>,
    pub goldens_passed: u32,
    pub goldens_total: u32,
    /// Milliseconds.
    pub check_ms: Option<u32>,
    /// RFC 3339.
    pub checked_at: String,
}

/// One installed version below `ATHENE_MODELS_DIR/athene-quality/`, as
/// its metadata says; the weights are never read for the list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AiModelVersion {
    /// The folder's name (`0.2.0-beta1`), what the admin picks.
    pub version: String,
    /// `contract_version` of `athene_model.json`, once readable.
    pub contract_version: Option<String>,
    /// The contract is `athene-quality-onnx-v1` and the metadata reads:
    /// the version may be picked.
    pub supported: bool,
    /// Why it may not be picked (`contract_unsupported`, `not_found`,
    /// `invalid_package`).
    pub problem: Option<AiModelError>,
    pub problem_detail: Option<String>,
    /// `release_ready` of the metadata.
    pub release_ready: Option<bool>,
    /// The folder's files, bytes.
    pub size_bytes: u64,
    /// `LICENSES.md` / `BASE_LICENSE.txt` ship with it.
    pub licenses: bool,
    /// Loaded and answering now.
    pub active: bool,
    /// The selected version ([`AiModelStatus::selected_version`]).
    pub selected: bool,
    pub last_check: Option<AiModelCheck>,
}

/// A pick of the admin on its way in, or the last one that failed (the
/// model before it stays active).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AiModelSwap {
    pub version: String,
    /// `checking` or `error`.
    pub state: AiModelState,
    pub error: Option<AiModelError>,
    pub error_detail: Option<String>,
    /// RFC 3339.
    pub started_at: String,
    /// RFC 3339; `None` while checking.
    pub finished_at: Option<String>,
}

/// What runs and what is installed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AiModelStatus {
    /// What answers the quality check now (where a workspace chose
    /// "Regeln + Athenos-Modell").
    pub engine: AiEngine,
    pub own_model: AiModelState,
    /// "Athenos-Modell: an / aus", as stored.
    pub enabled: bool,
    /// The admin decides, or the kill switch `ATHENE_QUALITY_MODEL=off`.
    pub control: AiModelControl,
    /// "Aktive Version": the version to run. The admin's pick once there
    /// is one; until then the start value.
    pub selected_version: String,
    /// The admin has picked a version (it wins over the start value).
    pub version_picked: bool,
    /// "Startwert aus Konfiguration": `ATHENE_QUALITY_MODEL` other than
    /// `off` (`athene-quality/0.4.1-beta3`), used while nothing is picked;
    /// `None`: the built-in `0.2.0-beta1`.
    pub env_default: Option<String>,
    /// The version loaded now; `None`: no model in memory.
    pub active_version: Option<String>,
    /// A pick being checked, or the last one that failed.
    pub swap: Option<AiModelSwap>,
    /// Every installed version, newest first.
    pub versions: Vec<AiModelVersion>,
    /// The active version (or, while none is, the selected one).
    pub quality_model: AiQualityModelStatus,
}

/// The platform's model as a workspace page shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AiWorkspaceModel {
    /// `0.2.0-beta1`.
    pub version: String,
    /// `release_ready` of the package.
    pub release_ready: bool,
}

/// One licence file of the model package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AiLicenseFile {
    /// `LICENSES.md` or `BASE_LICENSE.txt`.
    pub name: String,
    pub text: String,
}

/// `GET /admin/athene-ai/model/licenses`: the licence files shipped next
/// to the model, as they are on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AiModelLicensesResponse {
    pub version: String,
    pub files: Vec<AiLicenseFile>,
}

/// `GET /admin/athene-ai`, and the answer of every admin change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AtheneAiAdminResponse {
    /// The stored mode.
    pub visibility: AiVisibility,
    /// The mode in effect (`off` while the kill switch is off).
    pub effective: AiVisibility,
    /// The feature flag `module.ticket_check` (the hard kill switch) is on.
    pub kill_switch_on: bool,
    /// Beta requests are approved as they come in.
    pub beta_auto_approve: bool,
    /// Enrolled workspaces, by name.
    pub enrolled: Vec<AiBetaWorkspace>,
    /// Open requests, oldest first.
    pub requests: Vec<AiBetaWorkspace>,
    /// Every workspace of the instance (id and name), for the picker.
    pub workspaces: Vec<AiWorkspaceRef>,
    pub model: AiModelStatus,
    /// RFC 3339; `None` while never changed.
    pub updated_at: Option<String>,
}

/// `PUT /admin/athene-ai`. Absent fields stay as they are.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "ai/", optional_fields = nullable)]
pub struct UpdateAtheneAiRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<AiVisibility>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beta_auto_approve: Option<bool>,
    /// "Athenos-Modell: an / aus". Off unloads the model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_enabled: Option<bool>,
    /// "Aktive Version": an installed version's folder name. A new one is
    /// checked in the background and swapped in once it passed (`409`
    /// while another pick is being checked, `404` when not installed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1, max = 64))]
    pub model_version: Option<String>,
}

/// `GET /admin/athene-ai/model/licenses`: the version (the active one
/// when absent).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema, TS)]
#[ts(export_to = "ai/", optional_fields = nullable)]
pub struct AiModelLicensesQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1, max = 64))]
    pub version: Option<String>,
}

/// `POST /admin/athene-ai/workspaces`: enrol a workspace directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "ai/")]
pub struct EnrollAiWorkspaceRequest {
    #[schema(value_type = String)]
    #[ts(type = "string")]
    pub workspace_id: Uuid,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn wire_names_round_trip() {
        for v in AiVisibility::ALL {
            let json = serde_json::to_string(v).unwrap();
            assert_eq!(json, format!("\"{}\"", v.as_str()));
            assert_eq!(AiVisibility::parse(v.as_str()), Some(*v));
        }
        for s in AiEnrollmentStatus::ALL {
            assert_eq!(AiEnrollmentStatus::parse(s.as_str()), Some(*s));
        }
        assert_eq!(AiVisibility::parse("dev_only"), None);
        for e in AiQualityEngine::ALL {
            assert_eq!(AiQualityEngine::parse(e.as_str()), Some(*e));
        }
        assert_eq!(AiQualityEngine::default(), AiQualityEngine::RulesModel);
        assert_eq!(AiModelState::parse("ready"), Some(AiModelState::Ready));
        assert_eq!(AiModelControl::default(), AiModelControl::Admin);
        assert_eq!(
            AiModelControl::parse("env_off"),
            Some(AiModelControl::EnvOff)
        );
        assert_eq!(
            AiModelError::parse("golden_mismatch"),
            Some(AiModelError::GoldenMismatch)
        );
    }

    #[test]
    fn defaults_are_on_and_not_enrolled() {
        assert_eq!(AiVisibility::default(), AiVisibility::On);
        let a = AtheneAiAvailability::HIDDEN;
        assert!(!a.available && !a.can_enroll);
        assert_eq!(a.visibility, AiVisibility::Off);
        assert_eq!(a.enrollment, AiEnrollmentStatus::None);
    }

    #[test]
    fn update_refuses_unknown_fields() {
        let ok: UpdateAtheneAiRequest =
            serde_json::from_str(r#"{"visibility":"preview"}"#).unwrap();
        assert_eq!(ok.visibility, Some(AiVisibility::Preview));
        assert!(serde_json::from_str::<UpdateAtheneAiRequest>(r#"{"mode":"on"}"#).is_err());
        assert!(
            serde_json::from_str::<UpdateAtheneAiRequest>(r#"{"visibility":"dev_only"}"#).is_err()
        );
        let model: UpdateAtheneAiRequest =
            serde_json::from_str(r#"{"model_enabled":false,"model_version":"0.4.0"}"#).unwrap();
        assert_eq!(model.model_enabled, Some(false));
        assert_eq!(model.model_version.as_deref(), Some("0.4.0"));
        assert!(model.validate().is_ok());
        let long = UpdateAtheneAiRequest {
            model_version: Some("x".repeat(65)),
            ..UpdateAtheneAiRequest::default()
        };
        assert!(long.validate().is_err());
        // The platform-wide engine of Q020 is gone: workspaces choose it.
        assert!(
            serde_json::from_str::<UpdateAtheneAiRequest>(r#"{"quality_engine":"rules"}"#).is_err()
        );
        assert!(
            serde_json::from_str::<EnrollAiWorkspaceRequest>(r#"{"workspace_id":"x"}"#).is_err()
        );
    }
}
