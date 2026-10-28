//! What the public pages need before anyone signs in, and the person's
//! consent to the terms (W9).
//!
//! - `GET /public/site` ([`PublicSiteInfo`]): the beta banner, the legal
//!   texts' version and the two blanks the pages fill (hoster, effective
//!   date), and whether the installation still waits for its first
//!   administrator (`setup_required`, no **active** user yet).
//! - `GET /user/legal-consent` and `POST /user/legal-consent`
//!   ([`LegalConsentStatus`], [`AcceptLegalConsentRequest`]): which version
//!   the signed-in person accepted and whether the current one still needs
//!   their consent (bumping `legal_version` in the platform settings asks
//!   everyone again).
//!
//! A registration names the version it accepted
//! (`RegisterRequest::accepted_legal_version`); a missing or outdated one
//! is `400` [`LEGAL_CONSENT_REQUIRED_CODE`], and so is a consent to any
//! other version than the current one.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use validator::Validate;

/// The error code of a missing or outdated consent (`400`).
pub const LEGAL_CONSENT_REQUIRED_CODE: &str = "ATH-LEGAL-001";

/// The version of the legal texts a fresh installation starts with.
pub const LEGAL_VERSION_DEFAULT: &str = "2026-10-28";

/// The hoster a fresh installation names on its legal pages.
pub const LEGAL_HOSTER_DEFAULT: &str = "Infomaniak Network SA, Genf, Schweiz";

/// The longest version string (`^[A-Za-z0-9._-]{1,32}$`).
pub const LEGAL_VERSION_MAX_CHARS: usize = 32;

/// The longest hoster line.
pub const LEGAL_HOSTER_MAX_CHARS: usize = 200;

/// Whether `value` is a legal version: 1 to 32 of `A-Z a-z 0-9 . _ -`.
pub fn is_legal_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= LEGAL_VERSION_MAX_CHARS
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Whether `value` is a calendar day `YYYY-MM-DD` that exists.
pub fn is_effective_date(value: &str) -> bool {
    value.len() == 10 && chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}

/// The legal texts' version and the blanks of the pages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "legal/")]
pub struct LegalInfo {
    /// The current version; a person who accepted another one is asked
    /// again.
    pub version: String,
    /// Who hosts the service (`[HOSTER, LAND]` on the pages when `null`).
    pub hoster: Option<String>,
    /// `YYYY-MM-DD` the texts apply from (`[DATUM]` when `null`).
    pub effective_date: Option<String>,
}

/// `GET /public/site` (no sign-in).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "legal/")]
pub struct PublicSiteInfo {
    pub beta_banner_enabled: bool,
    pub legal: LegalInfo,
    /// No active user exists yet: the first administrator still has to be
    /// set up (`/setup`, with the setup token).
    pub setup_required: bool,
}

/// `GET` and `POST /user/legal-consent`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "legal/")]
pub struct LegalConsentStatus {
    pub current_version: String,
    /// The version the person accepted last, `null` when never.
    pub accepted_version: Option<String>,
    /// RFC 3339, when they accepted it.
    pub accepted_at: Option<String>,
    /// `accepted_version` is not the current version.
    pub required: bool,
}

/// `POST /user/legal-consent`: the person accepts `version`, which must be
/// the current one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "legal/")]
pub struct AcceptLegalConsentRequest {
    #[validate(length(min = 1, max = 32))]
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn versions_and_days_are_checked() {
        assert!(is_legal_version(LEGAL_VERSION_DEFAULT));
        assert!(is_legal_version("v1.2_beta-3"));
        assert!(!is_legal_version(""));
        assert!(!is_legal_version("2026 10 28"));
        assert!(!is_legal_version(&"a".repeat(33)));
        assert!(is_effective_date("2026-11-01"));
        assert!(!is_effective_date("2026-02-30"));
        assert!(!is_effective_date("1.11.2026"));
        assert!(LEGAL_HOSTER_DEFAULT.chars().count() <= LEGAL_HOSTER_MAX_CHARS);
    }

    #[test]
    fn a_consent_names_only_its_version() {
        let req: AcceptLegalConsentRequest =
            serde_json::from_value(json!({ "version": "2026-10-28" })).unwrap();
        assert_eq!(req.version, "2026-10-28");
        assert!(serde_json::from_value::<AcceptLegalConsentRequest>(
            json!({ "version": "x", "at": "now" })
        )
        .is_err());
    }
}
