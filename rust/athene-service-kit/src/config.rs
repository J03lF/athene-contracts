//! Where a service listens, and secrets from the environment or a file.
//!
//! - [`bind_addr`]: the host injects `FENRIR_SERVICE_ADDR`
//!   (`127.0.0.1:<port>`); it wins, as `ip:port`, as a bare `ip` (then with
//!   `FENRIR_SERVICE_PORT`) or as `[v6]`. Without it the service binds
//!   loopback only, and a value that is not an address falls back to
//!   loopback with a warning — never to every interface (S1, SEC-08).
//! - [`secret`]: a secret comes from `NAME` or from the file `NAME_FILE`
//!   names (0600, outside every repo, S2/S4), never both. Errors are
//!   values, not panics (BE-14), and never contain the secret.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// The listen address the host injects.
pub const SERVICE_ADDR_VAR: &str = "FENRIR_SERVICE_ADDR";
/// The listen port the host injects (used with a bare IP or no address).
pub const SERVICE_PORT_VAR: &str = "FENRIR_SERVICE_PORT";

/// A configuration value that cannot be used. Never holds a secret.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("ambiguous secret source: set either {direct} or {file}, not both")]
    Ambiguous { direct: String, file: String },
    #[error("cannot read {var} ('{path}'): {reason}")]
    Unreadable {
        var: String,
        path: String,
        reason: String,
    },
    #[error("{var} is empty")]
    Empty { var: String },
    #[error("{var}: {reason}")]
    Invalid { var: String, reason: String },
}

impl ConfigError {
    pub(crate) fn invalid(var: &str, reason: impl Into<String>) -> Self {
        Self::Invalid {
            var: var.to_string(),
            reason: reason.into(),
        }
    }
}

/// A secret value. `Debug` and `Display` never show it (S4).
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The value itself, for the one place that needs it.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

/// `addr` as a listen address: `ip:port`, `[v6]:port`, a bare `ip` or
/// `[v6]` (then with `port`). Unset or blank is loopback.
pub fn parse_bind_addr(addr: Option<&str>, port: u16) -> Result<SocketAddr, ConfigError> {
    let Some(raw) = addr.map(str::trim).filter(|raw| !raw.is_empty()) else {
        return Ok(loopback(port));
    };
    if let Ok(addr) = raw.parse::<SocketAddr>() {
        return Ok(addr);
    }
    let bare = raw
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(raw);
    if let Ok(ip) = bare.parse::<IpAddr>() {
        return Ok(SocketAddr::new(ip, port));
    }
    Err(ConfigError::invalid(
        SERVICE_ADDR_VAR,
        format!("'{raw}' is neither ip:port nor ip"),
    ))
}

/// [`parse_bind_addr`], with loopback (and a warning) for a value that is
/// not an address.
pub fn bind_addr_or_loopback(addr: Option<&str>, port: u16) -> SocketAddr {
    parse_bind_addr(addr, port).unwrap_or_else(|err| {
        let fallback = loopback(port);
        tracing::warn!(
            event = "config.bind_addr.invalid",
            error = %err,
            fallback = %fallback,
            "FENRIR_SERVICE_ADDR is not an address; binding loopback"
        );
        fallback
    })
}

/// The listen address from `FENRIR_SERVICE_ADDR` / `FENRIR_SERVICE_PORT`.
pub fn bind_addr(default_port: u16) -> SocketAddr {
    bind_addr_from(&|name| std::env::var(name).ok(), default_port)
}

/// [`bind_addr`] through `lookup` instead of the process environment.
pub fn bind_addr_from(lookup: &dyn Fn(&str) -> Option<String>, default_port: u16) -> SocketAddr {
    let port = service_port(lookup, default_port);
    bind_addr_or_loopback(lookup(SERVICE_ADDR_VAR).as_deref(), port)
}

/// `FENRIR_SERVICE_PORT`, or `default_port` when unset or not a port.
pub fn service_port(lookup: &dyn Fn(&str) -> Option<String>, default_port: u16) -> u16 {
    lookup(SERVICE_PORT_VAR)
        .and_then(|raw| raw.trim().parse::<u16>().ok())
        .unwrap_or(default_port)
}

fn loopback(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
}

/// The first secret of `names` (aliases, in order of preference) from the
/// process environment. See [`secret_from`].
pub fn secret(names: &[&str]) -> Result<Option<Secret>, ConfigError> {
    secret_from(&|name| std::env::var(name).ok(), names)
}

/// A secret through `lookup`:
///
/// - `NAME=<value>` (trimmed; blank counts as unset), or
/// - `NAME_FILE=<path>`: the file's content, trimmed; a blank path or an
///   empty file is an error, and so is a file that cannot be read.
///
/// The first alias that sets a value and the first that names a file are
/// taken; when both exist the source is ambiguous. `Ok(None)` when none of
/// the names is set.
pub fn secret_from(
    lookup: &dyn Fn(&str) -> Option<String>,
    names: &[&str],
) -> Result<Option<Secret>, ConfigError> {
    let direct = names.iter().find_map(|name| {
        lookup(name)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(|value| (*name, value))
    });
    let file = names.iter().find_map(|name| {
        let var = format!("{name}_FILE");
        lookup(&var).map(|path| (var, path))
    });
    match (direct, file) {
        (Some((direct, _)), Some((file, _))) => Err(ConfigError::Ambiguous {
            direct: direct.to_string(),
            file,
        }),
        (Some((_, value)), None) => Ok(Some(Secret(value))),
        (None, Some((var, path))) => read_secret_file(&var, &path).map(Some),
        (None, None) => Ok(None),
    }
}

fn read_secret_file(var: &str, path: &str) -> Result<Secret, ConfigError> {
    let path = path.trim();
    if path.is_empty() {
        return Err(ConfigError::Empty {
            var: var.to_string(),
        });
    }
    let raw = std::fs::read_to_string(path).map_err(|err| ConfigError::Unreadable {
        var: var.to_string(),
        path: path.to_string(),
        reason: err.kind().to_string(),
    })?;
    let value = raw.trim();
    if value.is_empty() {
        return Err(ConfigError::Invalid {
            var: var.to_string(),
            reason: format!("'{path}' holds no secret"),
        });
    }
    Ok(Secret(value.to_string()))
}

/// A boolean flag of `names` (first one set wins): `1/true/yes/on` or
/// `0/false/no/off`; anything else is an error rather than a guess.
pub fn flag_from(
    lookup: &dyn Fn(&str) -> Option<String>,
    names: &[&str],
) -> Result<Option<bool>, ConfigError> {
    let Some((name, raw)) = names
        .iter()
        .find_map(|name| lookup(name).map(|raw| (*name, raw)))
    else {
        return Ok(None);
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(Some(true)),
        "0" | "false" | "no" | "off" => Ok(Some(false)),
        _ => Err(ConfigError::invalid(name, "must be true or false")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name| map.get(name).cloned()
    }

    fn addr(raw: &str) -> SocketAddr {
        raw.parse().unwrap()
    }

    #[test]
    fn the_host_address_wins() {
        assert_eq!(
            parse_bind_addr(Some("127.0.0.1:41000"), 3001),
            Ok(addr("127.0.0.1:41000"))
        );
        assert_eq!(
            parse_bind_addr(Some("[::1]:41000"), 3001),
            Ok(addr("[::1]:41000"))
        );
    }

    #[test]
    fn a_bare_ip_takes_the_port() {
        assert_eq!(
            parse_bind_addr(Some("10.0.0.5"), 41000),
            Ok(addr("10.0.0.5:41000"))
        );
        assert_eq!(parse_bind_addr(Some("::1"), 41000), Ok(addr("[::1]:41000")));
        assert_eq!(
            parse_bind_addr(Some("[::1]"), 41000),
            Ok(addr("[::1]:41000"))
        );
    }

    #[test]
    fn without_an_address_only_loopback_is_bound() {
        assert_eq!(parse_bind_addr(None, 41000), Ok(addr("127.0.0.1:41000")));
        assert_eq!(
            parse_bind_addr(Some("  "), 41000),
            Ok(addr("127.0.0.1:41000"))
        );
        assert!(parse_bind_addr(Some("localhost:41000"), 1).is_err());
        assert_eq!(
            bind_addr_or_loopback(Some("not-an-address"), 3001),
            addr("127.0.0.1:3001")
        );
    }

    #[test]
    fn bind_addr_reads_the_host_variables() {
        let lookup = env(&[
            ("FENRIR_SERVICE_ADDR", "10.1.2.3"),
            ("FENRIR_SERVICE_PORT", "41005"),
        ]);
        assert_eq!(bind_addr_from(&lookup, 3004), addr("10.1.2.3:41005"));
        let lookup = env(&[("FENRIR_SERVICE_PORT", "bogus")]);
        assert_eq!(bind_addr_from(&lookup, 3004), addr("127.0.0.1:3004"));
    }

    fn scratch_file(content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("kit-secret-{}", uuid::Uuid::new_v4()));
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn a_secret_comes_from_the_variable_or_its_file() {
        let lookup = env(&[("TOKEN", "  direct-value  ")]);
        assert_eq!(
            secret_from(&lookup, &["TOKEN"]).unwrap().unwrap().expose(),
            "direct-value"
        );

        let path = scratch_file("from-file\n");
        let lookup = env(&[("TOKEN_FILE", path.to_str().unwrap())]);
        assert_eq!(
            secret_from(&lookup, &["TOKEN"]).unwrap().unwrap().expose(),
            "from-file"
        );
        std::fs::remove_file(path).unwrap();

        assert_eq!(secret_from(&env(&[]), &["TOKEN"]), Ok(None));
        // Blank means unset for the variable itself.
        assert_eq!(secret_from(&env(&[("TOKEN", " ")]), &["TOKEN"]), Ok(None));
    }

    #[test]
    fn aliases_are_tried_in_order() {
        let lookup = env(&[("B", "second"), ("A", "first")]);
        assert_eq!(
            secret_from(&lookup, &["A", "B"]).unwrap().unwrap().expose(),
            "first"
        );
        let lookup = env(&[("B", "second")]);
        assert_eq!(
            secret_from(&lookup, &["A", "B"]).unwrap().unwrap().expose(),
            "second"
        );
    }

    #[test]
    fn a_secret_source_that_cannot_be_used_is_an_error_without_the_value() {
        let path = scratch_file("x");
        let both = env(&[
            ("TOKEN", "value-123"),
            ("TOKEN_FILE", path.to_str().unwrap()),
        ]);
        let err = secret_from(&both, &["TOKEN"]).unwrap_err();
        assert!(matches!(err, ConfigError::Ambiguous { .. }));
        assert!(!err.to_string().contains("value-123"));
        std::fs::remove_file(&path).unwrap();

        let missing = env(&[("TOKEN_FILE", "/nonexistent/kit/secret")]);
        assert!(matches!(
            secret_from(&missing, &["TOKEN"]),
            Err(ConfigError::Unreadable { .. })
        ));
        let blank_path = env(&[("TOKEN_FILE", "  ")]);
        assert!(matches!(
            secret_from(&blank_path, &["TOKEN"]),
            Err(ConfigError::Empty { .. })
        ));
        let empty = scratch_file("\n  \n");
        let lookup = env(&[("TOKEN_FILE", empty.to_str().unwrap())]);
        assert!(matches!(
            secret_from(&lookup, &["TOKEN"]),
            Err(ConfigError::Invalid { .. })
        ));
        std::fs::remove_file(empty).unwrap();
    }

    #[test]
    fn secrets_never_print() {
        let secret = Secret::new("hunter2-hunter2");
        assert!(!format!("{secret:?}").contains("hunter2"));
        assert!(!format!("{secret}").contains("hunter2"));
    }

    #[test]
    fn flags_are_strict() {
        assert_eq!(flag_from(&env(&[("F", "TRUE")]), &["F"]), Ok(Some(true)));
        assert_eq!(flag_from(&env(&[("F", "off")]), &["F"]), Ok(Some(false)));
        assert_eq!(flag_from(&env(&[]), &["F"]), Ok(None));
        assert!(flag_from(&env(&[("F", "maybe")]), &["F"]).is_err());
        assert_eq!(flag_from(&env(&[("G", "1")]), &["F", "G"]), Ok(Some(true)));
    }
}
