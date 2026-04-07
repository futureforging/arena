//! Host-side WASI capability backends for [`verity-runtime`](crate).

mod vault_anthropic_local;

pub use vault_anthropic_local::VaultAnthropicLocalFile;
