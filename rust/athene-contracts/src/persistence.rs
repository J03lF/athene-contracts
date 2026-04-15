//! Database Persistence Utilities
//!
//! Shared traits and helpers for database serialization.

/// Trait for enums that can be serialized to/from database strings.
///
/// This trait provides a standard interface for converting enums to database-safe
/// string representations and parsing them back. It's designed to be used with
/// repository implementations that interact with SQL databases.
///
/// # Example
///
/// ```
/// use athene_contracts::persistence::DbEnum;
///
/// #[derive(Debug, PartialEq)]
/// pub enum UserRole {
///     Admin,
///     User,
/// }
///
/// impl DbEnum for UserRole {
///     fn from_db(value: Option<&str>) -> Result<Self, String> {
///         match value.unwrap_or("user") {
///             "admin" => Ok(UserRole::Admin),
///             "user" => Ok(UserRole::User),
///             other => Err(format!("invalid user role '{}'", other)),
///         }
///     }
///
///     fn to_db(&self) -> &'static str {
///         match self {
///             UserRole::Admin => "admin",
///             UserRole::User => "user",
///         }
///     }
/// }
/// ```
pub trait DbEnum: Sized {
    /// Parse enum from database string value.
    ///
    /// # Arguments
    ///
    /// * `value` - Optional string value from database column
    ///
    /// # Returns
    ///
    /// The parsed enum variant or an error message
    fn from_db(value: Option<&str>) -> Result<Self, String>;

    /// Convert enum to database string representation.
    ///
    /// # Returns
    ///
    /// Static string slice suitable for database storage
    fn to_db(&self) -> &'static str;
}
