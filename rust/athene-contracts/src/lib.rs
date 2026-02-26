//! Athene Contracts
//!
//! Shared DTOs that backend services and clients agree on.

pub mod common;
pub mod auth;

pub use common::{ApiError, ApiResponse, ListMeta, ListResponse};
