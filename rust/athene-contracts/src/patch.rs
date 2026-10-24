//! `Patch<T>`: one field of a partial update.
//!
//! A PATCH body has three things to say about a nullable field, and JSON
//! has exactly three ways to say them:
//!
//! | JSON                 | `Patch<T>`        | meaning                    |
//! |----------------------|-------------------|----------------------------|
//! | key missing          | `Patch::Absent`   | leave the value as it is   |
//! | `"key": null`        | `Patch::Null`     | clear the value            |
//! | `"key": <value>`     | `Patch::Value(v)` | set it to `v`              |
//!
//! `Option<Option<T>>` said the same with a hand-written deserializer per
//! field, and forgetting `skip_serializing_if` on one turned "missing" into
//! `null` on the way through the gateway: the value was deleted (BE-22).
//!
//! Every `Patch` field carries the same attribute, which makes serde and
//! the TypeScript agree:
//!
//! ```
//! use athene_contracts::Patch;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct UpdateThing {
//!     #[serde(default, skip_serializing_if = "Patch::is_absent")]
//!     due_date: Patch<String>,
//! }
//! ```
//!
//! - `default` turns a missing key into `Absent`. Without it, a missing key
//!   is a deserialisation error, so the omission cannot go unnoticed.
//! - `skip_serializing_if` writes `Absent` by leaving the key out.
//!   Serialising an `Absent` any other way is an error, never a silent
//!   `null`.
//! - ts-rs reads the pair as "may be omitted", so the field becomes
//!   `due_date?: string | null` (`T | null | undefined`).

use serde::de::{Deserialize, Deserializer};
use serde::ser::{Error as _, Serialize, Serializer};
use ts_rs::{Config, TypeVisitor, TS};

/// One field of a partial update: absent, explicitly `null`, or a value.
/// See the module docs for the attribute every field of this type needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Patch<T> {
    /// The key was not sent: leave the value alone.
    Absent,
    /// The key was sent as `null`: clear the value.
    Null,
    /// The key was sent with a value: set it.
    Value(T),
}

impl<T> Default for Patch<T> {
    fn default() -> Self {
        Self::Absent
    }
}

impl<T> Patch<T> {
    /// `true` when the key was not sent. Used as `skip_serializing_if`.
    pub const fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }

    /// `true` when the key was sent at all, as `null` or with a value.
    pub const fn is_present(&self) -> bool {
        !self.is_absent()
    }

    /// The value, when one was sent.
    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Value(value) => Some(value),
            Self::Absent | Self::Null => None,
        }
    }

    pub const fn as_ref(&self) -> Patch<&T> {
        match self {
            Self::Absent => Patch::Absent,
            Self::Null => Patch::Null,
            Self::Value(value) => Patch::Value(value),
        }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Patch<U> {
        match self {
            Self::Absent => Patch::Absent,
            Self::Null => Patch::Null,
            Self::Value(value) => Patch::Value(f(value)),
        }
    }

    /// Converts a sent value, keeping `Absent` and `Null` as they are; the
    /// first failure is returned. For parsing wire strings into domain types.
    pub fn try_map<U, E>(self, f: impl FnOnce(T) -> Result<U, E>) -> Result<Patch<U>, E> {
        Ok(match self {
            Self::Absent => Patch::Absent,
            Self::Null => Patch::Null,
            Self::Value(value) => Patch::Value(f(value)?),
        })
    }

    /// The domain's edit shape: `None` = leave alone, `Some(None)` = clear,
    /// `Some(Some(v))` = set.
    pub fn into_option(self) -> Option<Option<T>> {
        match self {
            Self::Absent => None,
            Self::Null => Some(None),
            Self::Value(value) => Some(Some(value)),
        }
    }
}

/// `None` = `Null`, `Some(v)` = `Value(v)`: an `Option` is always "sent".
impl<T> From<Option<T>> for Patch<T> {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Null, Self::Value)
    }
}

impl<T: Serialize> Serialize for Patch<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Value(value) => serializer.serialize_some(value),
            Self::Null => serializer.serialize_none(),
            // A missing key cannot be written by the value itself; writing
            // it as `null` would clear the field downstream.
            Self::Absent => Err(S::Error::custom(
                "Patch::Absent must be skipped: add \
                 `#[serde(default, skip_serializing_if = \"Patch::is_absent\")]` to the field",
            )),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Patch<T> {
    /// Only called for a key that is present; a missing key is `default`.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Option::<T>::deserialize(deserializer).map(Self::from)
    }
}

/// `T | null`, like `Option<T>`. The field's `default` +
/// `skip_serializing_if` make it optional, so it reads `key?: T | null`.
impl<T: TS> TS for Patch<T> {
    type WithoutGenerics = Patch<ts_rs::Dummy>;
    type OptionInnerType = T;
    // Lets `#[ts(optional_fields)]` on a struct treat it like an `Option`.
    const IS_OPTION: bool = true;

    fn name(cfg: &Config) -> String {
        format!("{} | null", T::name(cfg))
    }

    fn inline(cfg: &Config) -> String {
        format!("{} | null", T::inline(cfg))
    }

    fn visit_dependencies(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        T::visit_dependencies(v);
    }

    fn visit_generics(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        T::visit_generics(v);
        v.visit::<T>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, Default, PartialEq, Serialize, Deserialize, TS)]
    struct Edit {
        #[serde(default, skip_serializing_if = "Patch::is_absent")]
        due_date: Patch<String>,
        #[serde(default, skip_serializing_if = "Patch::is_absent")]
        points: Patch<i32>,
    }

    #[derive(Debug, Serialize)]
    struct Forgotten {
        due_date: Patch<String>,
    }

    fn edit(value: serde_json::Value) -> Edit {
        serde_json::from_value(value).expect("edit")
    }

    #[test]
    fn a_missing_key_is_absent() {
        assert_eq!(edit(json!({})), Edit::default());
        assert_eq!(edit(json!({})).due_date, Patch::Absent);
    }

    #[test]
    fn null_is_null() {
        assert_eq!(edit(json!({ "due_date": null })).due_date, Patch::Null);
    }

    #[test]
    fn a_value_is_a_value() {
        let e = edit(json!({ "due_date": "2026-11-01T12:00:00Z", "points": 3 }));
        assert_eq!(e.due_date, Patch::Value("2026-11-01T12:00:00Z".to_string()));
        assert_eq!(e.points, Patch::Value(3));
    }

    #[test]
    fn a_value_of_the_wrong_type_is_refused() {
        assert!(serde_json::from_value::<Edit>(json!({ "points": "three" })).is_err());
    }

    #[test]
    fn serialising_round_trips_all_three_states() {
        for body in [
            json!({}),
            json!({ "due_date": null }),
            json!({ "due_date": "2026-11-01", "points": null }),
            json!({ "points": 0 }),
        ] {
            let parsed = edit(body.clone());
            assert_eq!(serde_json::to_value(&parsed).expect("serialise"), body);
        }
    }

    #[test]
    fn an_absent_field_without_skip_is_an_error_not_a_null() {
        let err = serde_json::to_value(Forgotten {
            due_date: Patch::Absent,
        })
        .expect_err("Absent must not serialise as null");
        assert!(err.to_string().contains("skip_serializing_if"), "{err}");
        assert_eq!(
            serde_json::to_value(Forgotten {
                due_date: Patch::Null
            })
            .expect("null"),
            json!({ "due_date": null })
        );
    }

    #[test]
    fn a_json_value_patch_tells_null_from_absent() {
        #[derive(Deserialize)]
        struct WithJson {
            #[serde(default)]
            default_value: Patch<serde_json::Value>,
        }
        let parse = |v| {
            serde_json::from_value::<WithJson>(v)
                .expect("parse")
                .default_value
        };
        assert_eq!(parse(json!({})), Patch::Absent);
        assert_eq!(parse(json!({ "default_value": null })), Patch::Null);
        assert_eq!(
            parse(json!({ "default_value": { "a": 1 } })),
            Patch::Value(json!({ "a": 1 }))
        );
    }

    #[test]
    fn into_option_is_the_domain_edit_shape() {
        assert_eq!(Patch::<i32>::Absent.into_option(), None);
        assert_eq!(Patch::<i32>::Null.into_option(), Some(None));
        assert_eq!(Patch::Value(1).into_option(), Some(Some(1)));
    }

    #[test]
    fn try_map_keeps_absent_and_null_and_reports_the_failure() {
        let parse = |p: Patch<&str>| p.try_map(str::parse::<i32>);
        assert_eq!(parse(Patch::Absent), Ok(Patch::Absent));
        assert_eq!(parse(Patch::Null), Ok(Patch::Null));
        assert_eq!(parse(Patch::Value("7")), Ok(Patch::Value(7)));
        assert!(parse(Patch::Value("x")).is_err());
    }

    #[test]
    fn typescript_is_an_optional_nullable_field() {
        let cfg = Config::new();
        assert_eq!(<Patch<String> as TS>::name(&cfg), "string | null");
        assert_eq!(
            Edit::inline(&cfg),
            "{ due_date?: string | null, points?: number | null, }"
        );
    }
}
