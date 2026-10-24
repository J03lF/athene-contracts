//! Athene Contracts
//!
//! Shared DTOs that backend services and clients agree on.

pub mod activity;
pub mod admin;
pub mod auth;
pub mod common;
pub mod dashboard;
pub mod docs;
pub mod edit;
pub mod field_definitions;
pub mod patch;
pub mod persistence;
pub mod policy;
pub mod search;
pub mod teams;
pub mod ticket_check;
pub mod ticket_types;
pub mod user;
pub mod work;
pub mod workspace;

pub use common::{ApiError, ApiErrorCode, ApiErrorEnvelope, ApiResponse, ListMeta, ListResponse};
pub use patch::Patch;
pub use persistence::DbEnum;
