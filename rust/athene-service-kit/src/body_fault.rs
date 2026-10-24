//! What a request body that does not fit its contract type got wrong, for
//! the field-level details of a 400 — one copy for the core's and the
//! gateway's `ContractJson` / `ValidatedJson` (G1).

/// The field and the reason of a body's deserialization failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyFault {
    /// The field named by the failure (`title`, `fields[2].key`), when the
    /// failure names one; `None` for a body that is not JSON at all.
    pub field: Option<String>,
    /// `syntax`, `missing`, `unknown`, `duplicate`, `type`, `value`,
    /// `length` or `invalid`.
    pub reason: &'static str,
}

/// Classifies a body's deserialization failure at `path` (the
/// `serde_path_to_error` path, `.` for the top level).
///
/// serde reports which field is missing or unknown only in its message, so
/// this reads the start of it — serde's own fixed wording — and keeps the
/// field name. The message itself is never answered: it carries the
/// contract's Rust type names and the parser's positions.
pub fn body_fault(err: &serde_json::Error, path: &str) -> BodyFault {
    use serde_json::error::Category;
    if !matches!(err.classify(), Category::Data) {
        return BodyFault {
            field: None,
            reason: "syntax",
        };
    }
    let message = err.to_string();
    let named = |prefix: &str| -> Option<String> {
        let rest = message.strip_prefix(prefix)?;
        let end = rest.find('`')?;
        Some(rest[..end].to_string())
    };
    let parent = (path != ".").then_some(path);
    let join = |name: String| -> String {
        match parent {
            // `serde_path_to_error` may already have entered the field.
            Some(parent) if parent == name || parent.ends_with(&format!(".{name}")) => {
                parent.to_string()
            }
            Some(parent) => format!("{parent}.{name}"),
            None => name,
        }
    };
    for (prefix, reason) in [
        ("missing field `", "missing"),
        ("unknown field `", "unknown"),
        ("duplicate field `", "duplicate"),
    ] {
        if let Some(name) = named(prefix) {
            return BodyFault {
                field: Some(join(name)),
                reason,
            };
        }
    }
    let reason = if message.starts_with("invalid type") {
        "type"
    } else if message.starts_with("invalid value") || message.starts_with("unknown variant") {
        "value"
    } else if message.starts_with("invalid length") {
        "length"
    } else {
        "invalid"
    };
    BodyFault {
        field: parent.map(str::to_string),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the callers do: deserialize, and on failure take the path the
    /// error is at (here `.` or the top-level field, as the modules'
    /// `serde_path_to_error` reports it for these bodies).
    fn fault_of<T: serde::de::DeserializeOwned + std::fmt::Debug>(
        body: &str,
        path: &str,
    ) -> BodyFault {
        let err = serde_json::from_str::<T>(body).expect_err("refused");
        body_fault(&err, path)
    }

    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct Body {
        title: String,
        #[serde(default)]
        priority: Option<Priority>,
    }

    #[derive(Debug, serde::Deserialize)]
    #[serde(rename_all = "lowercase")]
    enum Priority {
        Low,
    }

    #[test]
    fn a_body_fault_names_the_field_and_the_reason() {
        let cases = [
            (r#"{}"#, ".", Some("title"), "missing"),
            (r#"{"title":"t","bogus":1}"#, ".", Some("bogus"), "unknown"),
            (r#"{"title":5}"#, "title", Some("title"), "type"),
            (
                r#"{"title":"t","priority":"urgent"}"#,
                "priority",
                Some("priority"),
                "value",
            ),
            (r#"{"title":"#, ".", None, "syntax"),
            (r#"not json"#, ".", None, "syntax"),
        ];
        for (body, path, field, reason) in cases {
            assert_eq!(
                fault_of::<Body>(body, path),
                BodyFault {
                    field: field.map(str::to_string),
                    reason
                },
                "{body}"
            );
        }
    }

    #[test]
    fn a_nested_field_is_joined_once() {
        let err = serde_json::from_str::<Body>(r#"{}"#).expect_err("refused");
        assert_eq!(
            body_fault(&err, "nested").field.as_deref(),
            Some("nested.title")
        );
        assert_eq!(
            body_fault(&err, "outer.title").field.as_deref(),
            Some("outer.title")
        );
    }
}
