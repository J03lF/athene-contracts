//! The operating mode every Athenos component runs in (W9A-03).
//!
//! One switch, `FENRIR_ENV`, which the Fenrir host always injects (`prod`
//! or `dev`). `prod` / `production` is production; `dev` / `development` /
//! `local` / `test` is dev. Anything else, and a missing value, is
//! production: dev conveniences must be asked for, never fallen into.
//!
//! A module may keep its own variable (`AUTH_ENV`, …), but it can only
//! tighten: [`RunMode::resolve`] turns a module override that asks for dev
//! under a production host into an error, so the module refuses to start
//! instead of quietly running in dev mode.

use std::fmt;

/// The host's mode variable.
pub const FENRIR_ENV_VAR: &str = "FENRIR_ENV";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Production,
    Dev,
}

impl RunMode {
    /// The mode a value names: dev only for the explicit dev names.
    pub fn parse(raw: Option<&str>) -> Self {
        match raw
            .map(|value| value.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("dev" | "development" | "local" | "test") => Self::Dev,
            _ => Self::Production,
        }
    }

    /// The host's mode, tightened by a module override (`override_vars`,
    /// first set wins): a production override under a dev host gives
    /// production; a dev override under a production host is an error.
    pub fn resolve(
        lookup: &dyn Fn(&str) -> Option<String>,
        override_vars: &[&str],
    ) -> Result<Self, ModeConflict> {
        let host = Self::parse(lookup(FENRIR_ENV_VAR).as_deref());
        let Some((var, value)) = override_vars.iter().find_map(|var| {
            lookup(var)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .map(|value| (*var, value))
        }) else {
            return Ok(host);
        };
        match (host, Self::parse(Some(&value))) {
            (Self::Production, Self::Dev) => Err(ModeConflict {
                var: var.to_string(),
            }),
            (_, Self::Production) => Ok(Self::Production),
            (Self::Dev, Self::Dev) => Ok(Self::Dev),
        }
    }

    /// [`RunMode::resolve`] over the process environment.
    pub fn from_env(override_vars: &[&str]) -> Result<Self, ModeConflict> {
        Self::resolve(&|name| std::env::var(name).ok(), override_vars)
    }

    pub fn is_production(self) -> bool {
        self == Self::Production
    }

    pub fn is_dev(self) -> bool {
        self == Self::Dev
    }

    /// `prod` or `dev`, the names the host injects.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Production => "prod",
            Self::Dev => "dev",
        }
    }
}

impl fmt::Display for RunMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A module variable asks for dev mode while the host runs in production.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeConflict {
    pub var: String,
}

impl fmt::Display for ModeConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} asks for dev mode, but {FENRIR_ENV_VAR} is production (missing or unknown \
             counts as production); a module may only tighten the host's mode",
            self.var
        )
    }
}

impl std::error::Error for ModeConflict {}

/// Dev-only settings found in a production configuration: the component
/// refuses to start and names every one of them (never their values).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevSettingsInProduction {
    pub component: &'static str,
    pub settings: Vec<String>,
}

impl fmt::Display for DevSettingsInProduction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} refuses to start in production with dev-only settings: {}",
            self.component,
            self.settings.join(", ")
        )
    }
}

impl std::error::Error for DevSettingsInProduction {}

/// `Err` naming `found` when it is not empty.
pub fn refuse_dev_settings(
    component: &'static str,
    found: Vec<String>,
) -> Result<(), DevSettingsInProduction> {
    if found.is_empty() {
        Ok(())
    } else {
        Err(DevSettingsInProduction {
            component,
            settings: found,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |key| map.get(key).cloned()
    }

    #[test]
    fn missing_or_unknown_is_production() {
        assert_eq!(RunMode::parse(None), RunMode::Production);
        assert_eq!(RunMode::parse(Some("")), RunMode::Production);
        assert_eq!(RunMode::parse(Some("staging")), RunMode::Production);
        assert_eq!(RunMode::parse(Some("prod")), RunMode::Production);
        assert_eq!(RunMode::parse(Some("Production")), RunMode::Production);
        for dev in ["dev", " DEV ", "development", "local", "test"] {
            assert_eq!(RunMode::parse(Some(dev)), RunMode::Dev, "{dev}");
        }
    }

    #[test]
    fn an_override_only_tightens() {
        let resolve = |pairs: &[(&str, &str)]| RunMode::resolve(&lookup(pairs), &["AUTH_ENV"]);
        assert_eq!(resolve(&[]), Ok(RunMode::Production));
        assert_eq!(resolve(&[("FENRIR_ENV", "dev")]), Ok(RunMode::Dev));
        assert_eq!(
            resolve(&[("FENRIR_ENV", "dev"), ("AUTH_ENV", "prod")]),
            Ok(RunMode::Production)
        );
        assert_eq!(
            resolve(&[("FENRIR_ENV", "dev"), ("AUTH_ENV", "dev")]),
            Ok(RunMode::Dev)
        );
        assert_eq!(
            resolve(&[("FENRIR_ENV", "dev"), ("AUTH_ENV", " ")]),
            Ok(RunMode::Dev)
        );
        let err = resolve(&[("FENRIR_ENV", "prod"), ("AUTH_ENV", "dev")]).unwrap_err();
        assert_eq!(err.var, "AUTH_ENV");
        // A missing host mode is production too: AUTH_ENV=dev cannot open it.
        assert!(resolve(&[("AUTH_ENV", "dev")]).is_err());
    }

    #[test]
    fn dev_settings_are_named_not_valued() {
        assert!(refuse_dev_settings("x", Vec::new()).is_ok());
        let err = refuse_dev_settings("auth-service", vec!["AUTH_DEV_PIN_SINK".into()])
            .unwrap_err()
            .to_string();
        assert!(err.contains("auth-service"));
        assert!(err.contains("AUTH_DEV_PIN_SINK"));
    }
}
