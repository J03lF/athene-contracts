//! Athene Contracts
//!
//! Shared DTOs that backend services and clients agree on.

pub mod admin;
pub mod auth;
pub mod common;
pub mod user;
pub mod workspace;

pub use common::{ApiError, ApiErrorCode, ApiErrorEnvelope, ApiResponse, ListMeta, ListResponse};
