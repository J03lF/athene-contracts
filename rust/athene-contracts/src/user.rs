use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct UserSettingsResponse {
    pub theme: String,
    pub palette: String,
    pub language: String,
    /// `cozy` | `compact`.
    #[serde(default)]
    pub density: String,
    pub shortcuts: serde_json::Value,
    pub dashboard_layout: Option<serde_json::Value>,
    /// Named filter sets the user has kept. Always a JSON array; the element
    /// shape is the client's own filter model, stored without interpretation.
    #[serde(default)]
    pub saved_views: serde_json::Value,
    /// `{ seen_at, archived }` — which inbox entries the user has dealt with.
    #[serde(default)]
    pub inbox_state: serde_json::Value,
    pub notifications_enabled: bool,
    pub email_notifications: bool,
    /// An IANA zone (`Europe/Berlin`); `None` = the browser's.
    #[serde(default)]
    pub timezone: Option<String>,
    /// The client's own preferences object.
    #[serde(default)]
    pub preferences: serde_json::Value,
}

/// A settings patch.
///
/// The API gateway deserialises the request body into this type and
/// re-serialises it towards the core, so this struct is a **filter**, not a
/// passthrough: a field it does not declare is dropped on the way through and
/// the caller gets a 200 for a write that never happened. Every field the
/// core accepts has to appear here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct UpdateUserSettingsRequest {
    pub theme: Option<String>,
    pub palette: Option<String>,
    pub language: Option<String>,
    pub density: Option<String>,
    pub shortcuts: Option<serde_json::Value>,
    pub dashboard_layout: Option<serde_json::Value>,
    pub saved_views: Option<serde_json::Value>,
    pub inbox_state: Option<serde_json::Value>,
    pub notifications_enabled: Option<bool>,
    pub email_notifications: Option<bool>,
    /// Absent = unchanged; `null` = back to the browser's zone (kept as two
    /// levels so a `null` survives the gateway).
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<String>)]
    pub timezone: Option<Option<String>>,
    /// Merged key by key into the stored object; a `null` value deletes
    /// that key. At most 16 KB once merged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub preferences: Option<serde_json::Map<String, serde_json::Value>>,
}

fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct UserProfileResponse {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub avatar_url: Option<String>,
    pub last_login_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
}
