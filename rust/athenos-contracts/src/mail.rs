//! Mail sending in the admin console (W6-E): `GET /admin/mail` shows
//! whether mail can go out and how it went last, `POST /admin/mail/test`
//! sends a test mail to the signed-in admin's own address (never to a
//! typed one: the console is no relay).
//!
//! The SMTP facts come from notification-service, which owns the relay;
//! the password is never part of any of these types, only where it came
//! from (`file`, `env`, `none`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use validator::Validate;

macro_rules! wire_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $( $(#[$vmeta:meta])* $variant:ident = $wire:literal ),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, TS)]
        #[ts(export_to = "mail/")]
        pub enum $name {
            $( $(#[$vmeta])* #[serde(rename = $wire)] $variant, )+
        }

        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self { $( Self::$variant => $wire, )+ }
            }
        }
    };
}

wire_enum! {
    /// How the connection to the relay is protected. There is no plain
    /// variant: without TLS nothing is sent.
    MailSecurity {
        /// Port 587 and others: STARTTLS, required.
        StartTls = "starttls",
        /// Port 465: TLS from the first byte.
        Tls = "tls",
    }
}

wire_enum! {
    /// Where the SMTP password was read from at start.
    MailPasswordSource {
        /// `SMTP_PASSWORD_FILE` (the intended way).
        File = "file",
        /// `SMTP_PASSWORD`.
        Env = "env",
        /// Neither: the relay is used without authentication.
        None = "none",
    }
}

wire_enum! {
    /// Why an address gets no mail.
    MailSkipReason {
        /// The account's address is not confirmed.
        Unverified = "unverified",
        /// The account is switched off.
        Inactive = "inactive",
        /// A domain that cannot receive mail (`.local`, `.test`, `example.com`, …).
        NonRoutable = "non_routable",
        /// Outside the recipient allowlist notification-service runs with.
        NotAllowed = "not_allowed",
    }
}

wire_enum! {
    /// What became of a test mail.
    TestMailOutcome {
        /// The relay accepted it.
        Sent = "sent",
        /// A temporary refusal: it is queued and tried again.
        Queued = "queued",
        /// Not sent, by rule (see `reason`).
        Skipped = "skipped",
        /// Not sent (see `error`).
        Failed = "failed",
    }
}

wire_enum! {
    /// The language of a test mail.
    MailLanguage {
        De = "de",
        En = "en",
    }
}

/// The relay as notification-service runs it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "mail/")]
pub struct MailTransportStatus {
    /// `SMTP_HOST` is set.
    pub configured: bool,
    /// Mail really leaves the box (SMTP); `false` is the dev sender that
    /// only logs.
    pub live: bool,
    pub host: Option<String>,
    pub port: Option<u16>,
    /// The SMTP user (no secret); the password is never shown.
    pub user: Option<String>,
    pub security: Option<MailSecurity>,
    pub password_source: MailPasswordSource,
    /// `"Athenos" <notify@mail.athenos.dev>`.
    pub from: String,
    /// Only these domains or addresses are mailed (empty: every routable one).
    pub allowlist: Vec<String>,
}

/// Mails waiting, in flight and given up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "mail/")]
pub struct MailQueueStatus {
    pub pending: u64,
    pub sending: u64,
    pub failed: u64,
}

/// The latest delivery error notification-service stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "mail/")]
pub struct MailLastError {
    /// RFC 3339.
    pub at: String,
    /// The relay's answer or the connection error, shortened.
    pub message: String,
}

/// What notification-service said (absent when it did not answer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "mail/")]
pub struct MailServiceStatus {
    pub transport: MailTransportStatus,
    pub queue: MailQueueStatus,
    /// RFC 3339; the latest mail the relay accepted.
    pub last_sent_at: Option<String>,
    pub last_error: Option<MailLastError>,
}

/// The core's side: whether notification mails are written at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "mail/")]
pub struct MailNotificationsStatus {
    /// `ATHENOS_NOTIFY_EMAIL` is not `off` (the kill switch).
    pub email_enabled: bool,
    /// The notification job runs (`ATHENOS_NOTIFY_INTERVAL_SECONDS` > 0).
    pub job_running: bool,
    /// What every link in a mail starts with (`ATHENOS_APP_URL`).
    pub app_url: String,
    /// One person gets at most one immediate mail per this many seconds;
    /// what happens meanwhile comes in the next one
    /// (`ATHENOS_NOTIFY_MIN_GAP_SECONDS`, `0`: no pause).
    pub min_gap_seconds: u64,
}

/// Whether the signed-in admin's own address gets mail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "mail/")]
pub struct MailRecipientStatus {
    pub email: String,
    /// Notifications reach this address (account and domain allow it).
    pub mailable: bool,
    pub reason: Option<MailSkipReason>,
}

/// `GET /admin/mail`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "mail/")]
pub struct MailStatusResponse {
    /// `None`: notification-service did not answer.
    pub service: Option<MailServiceStatus>,
    pub notifications: MailNotificationsStatus,
    pub you: MailRecipientStatus,
}

/// `POST /admin/mail/test`: a test mail to the signed-in admin.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Validate, ToSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "mail/", optional_fields)]
pub struct SendTestMailRequest {
    /// The mail's language; English without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<MailLanguage>,
}

/// The test mail's fate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export_to = "mail/")]
pub struct TestMailResponse {
    pub outcome: TestMailOutcome,
    /// Where it went (the admin's own address).
    pub to: String,
    pub reason: Option<MailSkipReason>,
    /// The relay's answer when it failed or was deferred.
    pub error: Option<String>,
}

/// Seconds between two test mails of one admin.
pub const TEST_MAIL_MIN_INTERVAL_SECONDS: u64 = 20;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_test_request_takes_only_a_language() {
        let empty: SendTestMailRequest = serde_json::from_value(json!({})).unwrap();
        assert_eq!(empty.language, None);
        let de: SendTestMailRequest = serde_json::from_value(json!({ "language": "de" })).unwrap();
        assert_eq!(de.language, Some(MailLanguage::De));
        assert!(serde_json::from_value::<SendTestMailRequest>(json!({ "to": "x@y.z" })).is_err());
        assert!(
            serde_json::from_value::<SendTestMailRequest>(json!({ "language": "fr" })).is_err()
        );
    }

    #[test]
    fn wire_names_are_stable() {
        assert_eq!(MailSecurity::StartTls.as_str(), "starttls");
        assert_eq!(MailPasswordSource::None.as_str(), "none");
        assert_eq!(MailSkipReason::NonRoutable.as_str(), "non_routable");
        assert_eq!(
            serde_json::to_value(TestMailOutcome::Queued).unwrap(),
            json!("queued")
        );
    }
}
