//! Who a mail may go to at all (W6-E).
//!
//! The one rule the core (before it queues a notification) and
//! notification-service (before it sends anything) both apply: an address
//! on a domain that cannot receive mail on the internet is never mailed.
//! Dev and test accounts live on such domains (`…@athenos.local`,
//! `…@example.com`); a real SMTP relay would bounce them, and every bounce
//! costs the sending domain reputation.
//!
//! Non-routable here means:
//! - the reserved and special-use names of RFC 2606 / RFC 6761 / RFC 6762
//!   and the de-facto private ones (`.local`, `.localhost`, `.test`,
//!   `.example`, `.invalid`, `.internal`, `.lan`, `.home.arpa`, …);
//! - `example.com`, `example.net`, `example.org` and their subdomains;
//! - a domain without a dot, an IP literal (`[…]`) or a malformed address.
//!
//! The check is about the domain only; whether the mailbox exists is the
//! receiving server's business (bounces come with the receiving wave).

use std::fmt;

/// Last labels that never route on the public internet.
const RESERVED_SUFFIXES: &[&str] = &[
    "local",
    "localhost",
    "localdomain",
    "test",
    "example",
    "invalid",
    "internal",
    "intranet",
    "private",
    "lan",
    "home",
    "corp",
    "home.arpa",
    "onion",
    "alt",
];

/// The documentation domains of RFC 2606 (with their subdomains).
const DOCUMENTATION_DOMAINS: &[&str] = &["example.com", "example.net", "example.org"];

/// Why an address is not mailed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unroutable {
    /// Not `local@domain`, or a character no mail domain has.
    Malformed,
    /// An IP literal (`user@[10.0.0.1]`) or a single label (`user@host`).
    NoPublicDomain,
    /// A reserved, special-use or documentation domain.
    ReservedDomain,
}

impl Unroutable {
    /// Short machine name for logs and metrics.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Malformed => "malformed",
            Self::NoPublicDomain => "no_public_domain",
            Self::ReservedDomain => "reserved_domain",
        }
    }
}

impl fmt::Display for Unroutable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The domain of `address`, lower-cased (`None` when there is no `@` with
/// something on both sides).
pub fn recipient_domain(address: &str) -> Option<String> {
    let (local, domain) = address.trim().rsplit_once('@')?;
    if local.is_empty() || domain.is_empty() {
        return None;
    }
    Some(domain.trim_end_matches('.').to_ascii_lowercase())
}

/// `Ok` when a mail to `address` can reach a public mail server.
pub fn check_recipient(address: &str) -> Result<(), Unroutable> {
    let domain = recipient_domain(address).ok_or(Unroutable::Malformed)?;
    if domain.starts_with('[') {
        return Err(Unroutable::NoPublicDomain);
    }
    let labels_ok = domain.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    });
    if !labels_ok || domain.len() > 253 {
        // Internationalised domains arrive as A-labels (`xn--…`) or not at
        // all: lettre and Mailgun both want ASCII here.
        return Err(Unroutable::Malformed);
    }
    let reserved = RESERVED_SUFFIXES
        .iter()
        .chain(DOCUMENTATION_DOMAINS)
        .any(|suffix| domain == *suffix || domain.ends_with(&format!(".{suffix}")));
    if reserved {
        return Err(Unroutable::ReservedDomain);
    }
    if !domain.contains('.') {
        return Err(Unroutable::NoPublicDomain);
    }
    Ok(())
}

/// Whether a mail to `address` can reach a public mail server.
pub fn is_routable(address: &str) -> bool {
    check_recipient(address).is_ok()
}

/// Whether `address` is on `allowlist` (lower-case domains such as
/// `athenos.dev`, which take their subdomains, or whole addresses). An
/// empty list allows everyone. notification-service's
/// `NOTIFY_RECIPIENT_ALLOWLIST`; the core asks the same for the admin's
/// own address.
pub fn listed(allowlist: &[String], address: &str) -> bool {
    if allowlist.is_empty() {
        return true;
    }
    let address = address.trim().to_ascii_lowercase();
    let domain = recipient_domain(&address).unwrap_or_default();
    allowlist.iter().any(|entry| {
        if entry.contains('@') {
            *entry == address
        } else {
            domain == *entry || domain.ends_with(&format!(".{entry}"))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_domains_are_routable() {
        for address in [
            "joel@athenos.dev",
            "Joel.Furter@Gmail.com",
            "a@mail.athenos.dev",
            "x+tag@sub.example-firm.ch",
            "a@xn--mnchen-3ya.de",
            "a@example.company",
            "a@examples.com",
        ] {
            assert_eq!(check_recipient(address), Ok(()), "{address}");
        }
    }

    #[test]
    fn dev_and_test_accounts_are_not() {
        for address in [
            "dev@athenos.local",
            "mira@ATHENOS.LOCAL",
            "e2e@athenos.local.",
            "a@localhost",
            "a@foo.localhost",
            "a@athenos.test",
            "a@athenos.example",
            "a@example.com",
            "a@mail.example.org",
            "a@example.net",
            "a@host.invalid",
            "a@box.internal",
            "a@nas.lan",
            "a@router.home.arpa",
            "a@x.onion",
        ] {
            assert_eq!(
                check_recipient(address),
                Err(Unroutable::ReservedDomain),
                "{address}"
            );
        }
    }

    #[test]
    fn single_labels_and_ip_literals_are_not() {
        for address in ["a@host", "a@[10.0.0.1]", "a@[IPv6:::1]"] {
            assert_eq!(
                check_recipient(address),
                Err(Unroutable::NoPublicDomain),
                "{address}"
            );
        }
    }

    #[test]
    fn malformed_addresses_are_not() {
        for address in [
            "",
            "no-at-sign",
            "@athenos.dev",
            "a@",
            "a@exa mple.com",
            "a@-bad.com",
            "a@bad-.com",
            "a@double..dot.com",
            "a@umlaut-ä.de",
            "a@semi;colon.com",
        ] {
            assert_eq!(
                check_recipient(address),
                Err(Unroutable::Malformed),
                "{address:?}"
            );
        }
        assert!(!is_routable("a@"));
    }

    #[test]
    fn the_allowlist_takes_domains_with_subdomains_and_addresses() {
        assert!(listed(&[], "a@b.ch"));
        let list = vec!["athenos.dev".to_string(), "joel@gmail.com".to_string()];
        assert!(listed(&list, "a@athenos.dev"));
        assert!(listed(&list, "a@mail.athenos.dev"));
        assert!(listed(&list, "Joel@Gmail.com"));
        assert!(!listed(&list, "x@gmail.com"));
        assert!(!listed(&list, "a@notathenos.dev"));
    }

    #[test]
    fn the_domain_is_read_lower_case_from_the_last_at() {
        assert_eq!(
            recipient_domain("\"a@b\"@Mail.Athenos.Dev").as_deref(),
            Some("mail.athenos.dev")
        );
        assert_eq!(recipient_domain("nobody"), None);
    }
}
