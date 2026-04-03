//! Host-side WASI capability backends for [`aria-runtime`](crate).

mod vault_anthropic_local;

pub use vault_anthropic_local::VaultAnthropicLocalFile;
