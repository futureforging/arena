//! Pluggable vault backends and related runtime helpers.

mod vault_anthropic_local;

pub use vault_anthropic_local::{
    VaultAnthropicLocalFile, ANTHROPIC_VAULT_LOCKER_ID, ANTHROPIC_VAULT_SECRET_ID,
};
