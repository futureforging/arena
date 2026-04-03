//! Pluggable vault backends, Omnia HTTP transport, and related runtime helpers.

mod omnia_wasi_http_post_json;
mod omnia_wasi_vault_anthropic_local;

pub use omnia_wasi_http_post_json::OmniaWasiHttpPostJson;
pub use omnia_wasi_vault_anthropic_local::{
    OmniaWasiVaultAnthropicLocal, ANTHROPIC_VAULT_LOCKER_ID, ANTHROPIC_VAULT_SECRET_ID,
};
