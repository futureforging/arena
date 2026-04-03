//! Omnia [`WasiVaultCtx`] that serves the Anthropic API key from a local file (read-only).

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use futures::FutureExt;
use omnia_wasi_vault::{FutureResult, Locker, WasiVaultCtx};

use crate::core::runtime::ANTHROPIC_API_KEY_SECRET;

/// Locker id expected by this vault backend.
pub const ANTHROPIC_VAULT_LOCKER_ID: &str = "aria-anthropic";

/// Secret id for the Anthropic API key (same string as [`ANTHROPIC_API_KEY_SECRET`](crate::core::runtime::ANTHROPIC_API_KEY_SECRET)).
pub const ANTHROPIC_VAULT_SECRET_ID: &str = ANTHROPIC_API_KEY_SECRET;

/// Default filename for the Anthropic API key at the repository root.
const DEFAULT_KEY_FILE_NAME: &str = "anthropic_api_key.txt";

/// Host vault backend that serves the Anthropic API key from a local file (read-only).
#[derive(Debug, Clone)]
pub struct OmniaWasiVaultAnthropicLocal {
    key_file: PathBuf,
}

impl OmniaWasiVaultAnthropicLocal {
    /// Creates a vault backend reading from the given file path.
    /// Pass [`None`] to use the default repo-root location.
    pub fn new(key_file: Option<PathBuf>) -> Self {
        let key_file = key_file
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_KEY_FILE_NAME));
        Self {
            key_file,
        }
    }
}

impl WasiVaultCtx for OmniaWasiVaultAnthropicLocal {
    fn open_locker(&self, identifier: String) -> FutureResult<Arc<dyn Locker>> {
        if identifier != ANTHROPIC_VAULT_LOCKER_ID {
            let expected = ANTHROPIC_VAULT_LOCKER_ID.to_string();
            return async move {
                Err(anyhow::anyhow!("unknown locker {identifier:?}; expected {expected:?}"))
            }
            .boxed();
        }
        let path = self
            .key_file
            .clone();
        async move {
            Ok(Arc::new(AnthropicFileLocker {
                path,
            }) as Arc<dyn Locker>)
        }
        .boxed()
    }
}

#[derive(Debug)]
struct AnthropicFileLocker {
    path: PathBuf,
}

impl Locker for AnthropicFileLocker {
    fn identifier(&self) -> String {
        ANTHROPIC_VAULT_LOCKER_ID.to_string()
    }

    fn get(&self, secret_id: String) -> FutureResult<Option<Vec<u8>>> {
        if secret_id != ANTHROPIC_VAULT_SECRET_ID {
            return async move { Ok(None) }.boxed();
        }
        let path = self
            .path
            .clone();
        async move {
            let raw = std::fs::read_to_string(&path)
                .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            Ok(Some(
                trimmed
                    .as_bytes()
                    .to_vec(),
            ))
        }
        .boxed()
    }

    fn set(&self, secret_id: String, _value: Vec<u8>) -> FutureResult<()> {
        async move { Err(anyhow::anyhow!("read-only vault: cannot set secret {secret_id:?}")) }
            .boxed()
    }

    fn delete(&self, secret_id: String) -> FutureResult<()> {
        async move { Err(anyhow::anyhow!("read-only vault: cannot delete secret {secret_id:?}")) }
            .boxed()
    }

    fn exists(&self, secret_id: String) -> FutureResult<bool> {
        if secret_id != ANTHROPIC_VAULT_SECRET_ID {
            return async move { Ok(false) }.boxed();
        }
        let path = self
            .path
            .clone();
        async move {
            match std::fs::read_to_string(&path) {
                Ok(raw) => Ok(!raw
                    .trim()
                    .is_empty()),
                Err(_) => Ok(false),
            }
        }
        .boxed()
    }

    fn list_ids(&self) -> FutureResult<Vec<String>> {
        let path = self
            .path
            .clone();
        async move {
            match std::fs::read_to_string(&path) {
                Ok(raw)
                    if !raw
                        .trim()
                        .is_empty() =>
                {
                    Ok(vec![ANTHROPIC_VAULT_SECRET_ID.to_string()])
                },
                _ => Ok(vec![]),
            }
        }
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use omnia_wasi_vault::{Locker, WasiVaultCtx};

    use super::{
        AnthropicFileLocker, OmniaWasiVaultAnthropicLocal, ANTHROPIC_VAULT_LOCKER_ID,
        ANTHROPIC_VAULT_SECRET_ID,
    };
    use crate::test_support::named_temp_file_with_writeln;

    #[tokio::test]
    async fn open_locker_rejects_unknown_id() -> Result<(), anyhow::Error> {
        let vault = OmniaWasiVaultAnthropicLocal::new(None);
        let outcome = vault
            .open_locker("not-aria-anthropic".to_string())
            .await;
        let err = match outcome {
            Ok(_) => {
                return Err(anyhow::anyhow!("expected open_locker to fail for unknown id"));
            },
            Err(e) => e,
        };
        assert!(
            err.to_string()
                .contains("unknown locker"),
            "unexpected: {err:?}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn open_locker_accepts_aria_anthropic() -> Result<(), anyhow::Error> {
        let tmp = named_temp_file_with_writeln("  key-from-test  ")?;
        let vault = OmniaWasiVaultAnthropicLocal::new(Some(
            tmp.path()
                .to_path_buf(),
        ));
        let locker = vault
            .open_locker(ANTHROPIC_VAULT_LOCKER_ID.to_string())
            .await?;
        assert_eq!(locker.identifier(), ANTHROPIC_VAULT_LOCKER_ID);
        Ok(())
    }

    #[tokio::test]
    async fn get_unknown_secret_returns_none() -> Result<(), anyhow::Error> {
        let tmp = named_temp_file_with_writeln("secret")?;
        let locker = AnthropicFileLocker {
            path: tmp
                .path()
                .to_path_buf(),
        };
        let got = locker
            .get("other".to_string())
            .await?;
        assert_eq!(got, None);
        Ok(())
    }

    #[tokio::test]
    async fn get_anthropic_key_reads_temp_file() -> Result<(), anyhow::Error> {
        let tmp = named_temp_file_with_writeln("  trimmed-key  ")?;
        let locker = AnthropicFileLocker {
            path: tmp
                .path()
                .to_path_buf(),
        };
        let got = locker
            .get(ANTHROPIC_VAULT_SECRET_ID.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("expected Some"))?;
        assert_eq!(got, b"trimmed-key");
        Ok(())
    }

    #[tokio::test]
    async fn set_returns_read_only_error() -> Result<(), anyhow::Error> {
        let tmp = named_temp_file_with_writeln("k")?;
        let locker = AnthropicFileLocker {
            path: tmp
                .path()
                .to_path_buf(),
        };
        let outcome = locker
            .set(ANTHROPIC_VAULT_SECRET_ID.to_string(), vec![0u8])
            .await;
        let err = match outcome {
            Ok(_) => return Err(anyhow::anyhow!("expected set to return Err")),
            Err(e) => e,
        };
        assert!(
            err.to_string()
                .contains("read-only"),
            "unexpected: {err:?}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn delete_returns_read_only_error() -> Result<(), anyhow::Error> {
        let tmp = named_temp_file_with_writeln("k")?;
        let locker = AnthropicFileLocker {
            path: tmp
                .path()
                .to_path_buf(),
        };
        let outcome = locker
            .delete(ANTHROPIC_VAULT_SECRET_ID.to_string())
            .await;
        let err = match outcome {
            Ok(_) => return Err(anyhow::anyhow!("expected delete to return Err")),
            Err(e) => e,
        };
        assert!(
            err.to_string()
                .contains("read-only"),
            "unexpected: {err:?}"
        );
        Ok(())
    }
}
