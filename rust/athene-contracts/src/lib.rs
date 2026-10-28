//! Athene Contracts
//!
//! Shared DTOs that backend services and clients agree on.

pub mod activity;
pub mod admin;
pub mod athene_ai;
pub mod attachments;
pub mod auth;
pub mod calendar;
pub mod common;
pub mod dashboard;
pub mod docs;
pub mod edit;
pub mod field_assignments;
pub mod field_definitions;
pub mod limits;
pub mod listing;
pub mod mail;
pub mod notifications;
pub mod overrides;
pub mod patch;
pub mod persistence;
pub mod policy;
pub mod project_stats;
pub mod queues;
pub mod release_notes;
pub mod roadmap;
pub mod roles;
pub mod search;
pub mod status_rules;
pub mod teams;
pub mod ticket_changes;
pub mod ticket_check;
pub mod ticket_types;
pub mod timer;
pub mod user;
pub mod work;
pub mod workflow;
pub mod workspace;

pub use common::{ApiError, ApiErrorCode, ApiErrorEnvelope, ApiResponse, ListMeta, ListResponse};
pub use patch::Patch;
pub use persistence::DbEnum;
