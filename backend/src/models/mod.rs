//! # API Models
//!
//! This module defines the request and response structures for the REST API.
//! These are separate from database models to allow API-specific formatting.
//!
//! ## Organization
//!
//! - `requests.rs` - Incoming request bodies
//! - `responses.rs` - Outgoing response bodies
//!
//! ## Serialization
//!
//! All models use Serde for JSON serialization/deserialization.
//! Field names are converted to camelCase for JavaScript clients.

pub mod requests;
pub mod responses;

pub use requests::*;
pub use responses::*;

