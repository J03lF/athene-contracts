//! Athene Contracts
//!
//! Shared DTOs that backend services and clients agree on.

pub mod common;
pub mod auth;

pub use common::{
    ApiError, ApiErrorCode, ApiErrorEnvelope, ApiResponse, ListMeta, ListResponse,
};
