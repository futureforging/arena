//! Helpers for loading secrets from the filesystem.
//!
//! See [`anthropic_api_key_from_local_file`].

mod anthropic_api_key_from_local_file;

pub use anthropic_api_key_from_local_file::{
    anthropic_api_key_from_local_file, AnthropicApiKeyError,
};
