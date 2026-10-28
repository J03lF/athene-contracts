//! The building blocks of a conflict-aware edit (step 3.7,
//! `phase3/CONFLICTS.md`).
//!
//! An edit names the fields it changes (the patch) and, per changed field,
//! the value the client based the change on (`expected`). The server writes
//! only those fields and only while each of them still holds its expected
//! value; another user's change to a *different* field is merged, a change
//! to the *same* field is a `409` [`EDIT_CONFLICT_CODE`] whose details
//! ([`EditConflictDetails`]) carry the current entity and the fields that
//! moved.
//!
//! - Simple and short-text fields expect their value.
//! - Long texts expect a [`TextHash`] of the text the client started from.
//! - Sets (labels, watchers, multi-value custom fields) take operations,
//!   [`SetOps`], and never conflict.
//! - "Save anyway" is the same request with `expected` set to the current
//!   value from the 409. There is no force flag.
//!
//! Rollout stage A: `expected` is optional; an edit without it is written
//! as before and counted (`edit_without_expected` in the core's log).
//! Stage B makes it required for the public edit endpoints.

use serde::de::{Deserializer, Error as _};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use ts_rs::TS;

/// The code of the `409` an edit with a stale base answers.
pub const EDIT_CONFLICT_CODE: &str = "ATH-RES-003";

/// The i18n key a client renders for [`EDIT_CONFLICT_CODE`].
pub const EDIT_CONFLICT_MESSAGE_KEY: &str = "errors.edit_conflict";

/// Operations on a set-valued field: what to add, what to remove.
///
/// Two people adding different labels at the same time both keep theirs;
/// an element named in both lists is a 400. An empty `SetOps` changes
/// nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "edit/")]
pub struct SetOps<T> {
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<T>>", optional)]
    pub add: Vec<T>,
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<T>>", optional)]
    pub remove: Vec<T>,
}

impl<T> Default for SetOps<T> {
    fn default() -> Self {
        Self {
            add: Vec::new(),
            remove: Vec::new(),
        }
    }
}

impl<T: PartialEq> SetOps<T> {
    /// `true` when there is nothing to add or remove.
    pub fn is_empty(&self) -> bool {
        self.add.is_empty() && self.remove.is_empty()
    }

    /// The first element that is both added and removed, if any.
    pub fn contradiction(&self) -> Option<&T> {
        self.add.iter().find(|item| self.remove.contains(item))
    }
}

impl<T> SetOps<T> {
    /// Converts every element, keeping the order; the first failure wins.
    pub fn try_map<U, E>(self, mut f: impl FnMut(T) -> Result<U, E>) -> Result<SetOps<U>, E> {
        Ok(SetOps {
            add: self.add.into_iter().map(&mut f).collect::<Result<_, _>>()?,
            remove: self
                .remove
                .into_iter()
                .map(&mut f)
                .collect::<Result<_, _>>()?,
        })
    }
}

/// SHA-256 of a long text, lower-case hex, as a long-text field's base.
///
/// The text is hashed as UTF-8 after normalising line endings (`\r\n` →
/// `\n`), nothing else: no trimming, no Unicode normalisation. The core
/// computes the same hash in SQL over the stored text, so a client must hash
/// exactly the text the server sent it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, TS)]
#[ts(export_to = "edit/", type = "string")]
pub struct TextHash(String);

impl TextHash {
    /// Length of the hex digest.
    pub const LEN: usize = 64;

    /// A hash in its wire form, or `None` when `hex` is not 64 lower-case
    /// hex digits.
    pub fn parse(hex: &str) -> Option<Self> {
        let valid = hex.len() == Self::LEN
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        valid.then(|| Self(hex.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for TextHash {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for TextHash {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).ok_or_else(|| {
            D::Error::invalid_value(
                serde::de::Unexpected::Str(&raw),
                &"a SHA-256 digest as 64 lower-case hex digits",
            )
        })
    }
}

/// `details` of a `409` [`EDIT_CONFLICT_CODE`].
///
/// `current` is the entity as it is now (for a long text the full text),
/// so the client can show it and offer "keep theirs" / "save anyway"
/// without another request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "edit/")]
pub struct EditConflictDetails<T> {
    pub current: T,
    pub fields: Vec<FieldConflict>,
}

/// One field whose base no longer holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "edit/")]
pub struct FieldConflict {
    /// The field's name in the patch (`status`, `assignee_id`, `title`,
    /// `custom_values.<key>`).
    pub field: String,
    /// The field's value now, in its wire form (for a long text the text).
    #[ts(type = "unknown")]
    pub current: JsonValue,
    /// Who changed it last, when the activity feed recorded it. Absent: the
    /// client says only that it was changed in the meantime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub changed_by: Option<ChangedBy>,
    /// When that change was recorded (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub changed_at: Option<String>,
}

/// The person behind the change that moved a field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "edit/")]
pub struct ChangedBy {
    pub id: String,
    pub display_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_ops_default_to_empty_and_reject_unknown_fields() {
        let ops: SetOps<String> = serde_json::from_str(r#"{"add":["a"]}"#).unwrap_or_default();
        assert_eq!(ops.add, vec!["a".to_string()]);
        assert!(ops.remove.is_empty());
        assert!(serde_json::from_str::<SetOps<String>>(r#"{"put":["a"]}"#).is_err());
        let empty: SetOps<String> = serde_json::from_str("{}").unwrap_or_default();
        assert!(empty.is_empty());
        assert_eq!(serde_json::to_string(&empty).unwrap_or_default(), "{}");
    }

    #[test]
    fn a_contradiction_is_found() {
        let ops = SetOps {
            add: vec![1, 2],
            remove: vec![3, 2],
        };
        assert_eq!(ops.contradiction(), Some(&2));
    }

    #[test]
    fn a_text_hash_is_64_lower_case_hex_digits() {
        let good = "a".repeat(64);
        assert!(serde_json::from_value::<TextHash>(JsonValue::String(good.clone())).is_ok());
        for bad in [
            "A".repeat(64),
            "a".repeat(63),
            "g".repeat(64),
            String::new(),
        ] {
            assert!(
                serde_json::from_value::<TextHash>(JsonValue::String(bad.clone())).is_err(),
                "{bad}"
            );
        }
        assert_eq!(
            serde_json::to_value(TextHash::parse(&good).unwrap_or_else(|| TextHash(good.clone())))
                .unwrap_or_default(),
            JsonValue::String(good)
        );
    }
}
