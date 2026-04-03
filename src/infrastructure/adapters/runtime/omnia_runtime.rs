//! [`Runtime`](crate::core::runtime::Runtime) backed by Omnia `wasi:vault` host traits.

use omnia_wasi_vault::WasiVaultCtx;

use crate::core::runtime::{Runtime, RuntimeError};

/// [`Runtime`] backed by an Omnia `wasi:vault` provider.
///
/// Delegates `get_secret` to the vault's locker interface. The vault backend
/// is injected at construction — it could be a local file reader, an in-memory
/// store, or a production secrets manager.
pub struct OmniaRuntime {
    vault: Box<dyn WasiVaultCtx>,
    locker_id: String,
    rt: tokio::runtime::Runtime,
}

impl OmniaRuntime {
    /// Creates an Omnia-backed runtime with the given vault backend and locker ID.
    ///
    /// Returns [`RuntimeError::Other`] if the tokio runtime cannot be created.
    pub fn new(
        vault: Box<dyn WasiVaultCtx>,
        locker_id: impl Into<String>,
    ) -> Result<Self, RuntimeError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| RuntimeError::Other(format!("failed to create async runtime: {e}")))?;
        Ok(Self {
            vault,
            locker_id: locker_id.into(),
            rt,
        })
    }
}

impl Runtime for OmniaRuntime {
    fn get_secret(&self, name: &str) -> Result<String, RuntimeError> {
        let locker_id = self
            .locker_id
            .clone();
        let secret_id = name.to_string();

        self.rt
            .block_on(async {
                let locker = self
                    .vault
                    .open_locker(locker_id)
                    .await
                    .map_err(|e| RuntimeError::Other(e.to_string()))?;

                let bytes = locker
                    .get(secret_id.clone())
                    .await
                    .map_err(|e| RuntimeError::Other(e.to_string()))?
                    .ok_or(RuntimeError::NotFound(secret_id))?;

                String::from_utf8(bytes)
                    .map_err(|e| RuntimeError::Other(format!("secret is not valid UTF-8: {e}")))
            })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::OmniaRuntime;
    use crate::{
        core::runtime::{Runtime, RuntimeError},
        infrastructure::adapters::runtime::{
            VaultAnthropicLocalFile, ANTHROPIC_VAULT_LOCKER_ID, ANTHROPIC_VAULT_SECRET_ID,
        },
    };

    #[test]
    fn get_secret_reads_via_vault_chain() -> Result<(), std::io::Error> {
        let mut tmp = tempfile::NamedTempFile::new()?;
        writeln!(tmp, "  omnia-key  ")?;
        let vault = Box::new(VaultAnthropicLocalFile::new(Some(
            tmp.path()
                .to_path_buf(),
        )));
        let runtime = OmniaRuntime::new(vault, ANTHROPIC_VAULT_LOCKER_ID)
            .map_err(|e| std::io::Error::other(format!("{e:?}")))?;
        let key = runtime
            .get_secret(ANTHROPIC_VAULT_SECRET_ID)
            .map_err(|e| std::io::Error::other(format!("{e:?}")))?;
        assert_eq!(key, "omnia-key");
        Ok(())
    }

    #[test]
    fn get_secret_unknown_name_returns_not_found() -> Result<(), std::io::Error> {
        let mut tmp = tempfile::NamedTempFile::new()?;
        writeln!(tmp, "k")?;
        let vault = Box::new(VaultAnthropicLocalFile::new(Some(
            tmp.path()
                .to_path_buf(),
        )));
        let runtime = OmniaRuntime::new(vault, ANTHROPIC_VAULT_LOCKER_ID)
            .map_err(|e| std::io::Error::other(format!("{e:?}")))?;
        let err = match runtime.get_secret("no_such_secret") {
            Ok(_) => {
                return Err(std::io::Error::other("expected NotFound for unknown secret name"));
            },
            Err(e) => e,
        };
        assert_eq!(err, RuntimeError::NotFound("no_such_secret".to_string()));
        Ok(())
    }
}
