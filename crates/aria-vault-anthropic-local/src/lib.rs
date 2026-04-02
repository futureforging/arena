//! Anemic **local-file vault** for the Anthropic API key: a single locker, a single secret id, one
//! on-disk file (`anthropic_api_key.txt` by default at the repo root), read-only `set`/`delete` on
//! the Omnia adapter. There is **no** Omnia runtime binary in this repository; [`VaultAnthropicLocalFile`]
//! exists for future wasmtime host wiring.
//!
//! ## Surfaces
//!
//! - **Native sync API:** [`anthropic_api_key_from_local_file`] for host binaries (e.g. `main`).
//! - **Omnia host adapter:** [`VaultAnthropicLocalFile`](vault_local::VaultAnthropicLocalFile) (`Backend` + `WasiVaultCtx`).
//!
//! ## Vault contract (future guests)
//!
//! - **Locker id:** [`ANTHROPIC_VAULT_LOCKER_ID`]
//! - **Secret id:** [`ANTHROPIC_VAULT_SECRET_ID`]
//!
//! Read/trim semantics for the key file are shared between the sync API and the Omnia locker (internal module `key_source`).

mod anthropic_api_key;
mod key_source;
mod vault_local;

pub use anthropic_api_key::{anthropic_api_key_from_local_file, AnthropicApiKeyError};
pub use vault_local::{
    AnthropicVaultConnectOptions, VaultAnthropicLocalFile, ANTHROPIC_VAULT_LOCKER_ID,
    ANTHROPIC_VAULT_SECRET_ID,
};
