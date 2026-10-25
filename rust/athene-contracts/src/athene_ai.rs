//! Athene AI: one brand and one place to control every intelligence
//! function of Athene. In R1 that is the ticket check (quality hints,
//! secret protection, "Ähnliche Tickets", local algorithms only); R2 adds
//! Athene's own local model.
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
    /// Who sees Athene AI on this instance.
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
    /// What answers: R1 runs algorithms only.
    AiEngine {
        AlgorithmsR1 = "algorithms_r1",
    }
}

ai_enum! {
    /// Athene's own model (R2).
    AiModelState {
        NotInstalled = "not_installed",
        Installed = "installed",
    }
}

/// `athene_ai` of the capabilities: whether Athene AI is there for the
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

/// One number pair of this week's Athene AI on a workspace page.
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
    /// This week's numbers; `None` while Athene AI is not available here.
    pub stats: Option<AiWeekStats>,
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

/// What runs and what is installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "ai/")]
pub struct AiModelStatus {
    pub engine: AiEngine,
    pub own_model: AiModelState,
}

/// `GET /admin/athene-ai`, and the answer of every admin change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
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
        assert!(
            serde_json::from_str::<EnrollAiWorkspaceRequest>(r#"{"workspace_id":"x"}"#).is_err()
        );
    }
}
