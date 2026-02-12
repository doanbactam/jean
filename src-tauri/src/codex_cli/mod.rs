//! Codex CLI management module
//!
//! Provides installation, version checking, and authentication management
//! for the OpenAI Codex CLI tool.

pub mod commands;
pub mod config;

pub use commands::*;
pub use config::*;
