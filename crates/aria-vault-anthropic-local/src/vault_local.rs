//! Omnia `wasi-vault` backend: one locker backed by `anthropic_api_key.txt` (read-only).

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Result;
use futures::FutureExt;
use omnia::Backend;
use omnia_wasi_vault::{FutureResult, Locker, WasiVaultCtx};
use tracing::instrument;

use crate::key_source::{
    default_repo_root_key_file, read_anthropic_key_strict, AnthropicApiKeyError,
};

/// Environment variable for the API key file path. If unset, the default repository-root file is used.
pub const ARIA_ANTHROPIC_API_KEY_FILE_ENV: &str = "ARIA_ANTHROPIC_API_KEY_FILE";

/// Locker id expected by [`VaultAnthropicLocalFile`] (pass to `vault::open` in guests).
pub const ANTHROPIC_VAULT_LOCKER_ID: &str = "aria-anthropic";

/// Secret id for the trimmed API key bytes (pass to `locker::get` in guests).
pub const ANTHROPIC_VAULT_SECRET_ID: &str = "anthropic_api_key";

/// Connection options for [`VaultAnthropicLocalFile`].
#[derive(Debug, Clone)]
pub struct AnthropicVaultConnectOptions {
    /// Path to `anthropic_api_key.txt` (or equivalent).
    pub key_file: PathBuf,
}

impl omnia::FromEnv for AnthropicVaultConnectOptions {
    fn from_env() -> Result<Self> {
        let key_file = match std::env::var(ARIA_ANTHROPIC_API_KEY_FILE_ENV) {
            Ok(p) => PathBuf::from(p),
            Err(_) => default_repo_root_key_file(),
        };
        Ok(Self {
            key_file,
        })
    }
}

/// Host vault backend that serves the Anthropic API key from a local file (read-only).
#[derive(Debug, Clone)]
pub struct VaultAnthropicLocalFile {
    key_file: PathBuf,
}

impl Backend for VaultAnthropicLocalFile {
    type ConnectOptions = AnthropicVaultConnectOptions;

    #[instrument(skip(options))]
    async fn connect_with(options: Self::ConnectOptions) -> Result<Self> {
        tracing::debug!(
            key_file = %options.key_file.display(),
            "initializing anthropic local-file vault"
        );
        Ok(Self {
            key_file: options.key_file,
        })
    }
}

impl WasiVaultCtx for VaultAnthropicLocalFile {
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

fn read_key_bytes(path: &Path) -> Result<Option<Vec<u8>>> {
    match read_anthropic_key_strict(path) {
        Ok(s) => Ok(Some(s.into_bytes())),
        Err(AnthropicApiKeyError::Read {
            source,
            ..
        }) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(AnthropicApiKeyError::Empty(_)) => Ok(None),
        Err(AnthropicApiKeyError::Read {
            path,
            source,
        }) => Err(anyhow::anyhow!("{}: {source}", path.display())),
    }
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
        async move { read_key_bytes(&path) }.boxed()
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
        let path = self
            .path
            .clone();
        async move {
            if secret_id != ANTHROPIC_VAULT_SECRET_ID {
                return Ok(false);
            }
            Ok(read_key_bytes(&path)?.is_some())
        }
        .boxed()
    }

    fn list_ids(&self) -> FutureResult<Vec<String>> {
        let path = self
            .path
            .clone();
        async move {
            if read_key_bytes(&path)?.is_some() {
                Ok(vec![ANTHROPIC_VAULT_SECRET_ID.to_string()])
            } else {
                Ok(vec![])
            }
        }
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[tokio::test]
    async fn open_wrong_locker_fails() {
        let v = VaultAnthropicLocalFile::connect_with(AnthropicVaultConnectOptions {
            key_file: PathBuf::from("/dev/null"),
        })
        .await
        .expect("connect");
        let err = v
            .open_locker("other".to_string())
            .await
            .expect_err("unknown locker");
        assert!(err
            .to_string()
            .contains("unknown locker"));
    }

    #[tokio::test]
    async fn get_round_trip() {
        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
        writeln!(tmp, "  sk-ant-test  ").expect("write");
        let path = tmp
            .path()
            .to_path_buf();

        let v = VaultAnthropicLocalFile::connect_with(AnthropicVaultConnectOptions {
            key_file: path,
        })
        .await
        .expect("connect");

        let locker = v
            .open_locker(ANTHROPIC_VAULT_LOCKER_ID.to_string())
            .await
            .expect("open locker");

        let got = locker
            .get(ANTHROPIC_VAULT_SECRET_ID.to_string())
            .await
            .expect("get");
        assert_eq!(got, Some(b"sk-ant-test".to_vec()));
    }
}
