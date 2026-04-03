//! [`Runtime`](aria_core::runtime::Runtime) backed by Omnia `wasi:vault` host traits.

use aria_core::{
    runtime::{Runtime, RuntimeError},
    transport::{BoxedPostJsonTransport, TransportError},
};
use omnia_wasi_vault::WasiVaultCtx;
use serde_json::Value;

use super::plugins::OmniaWasiHttpPostJson;

/// [`Runtime`] backed by an Omnia `wasi:vault` provider.
///
/// Delegates `get_secret` to the vault's locker interface. The vault backend
/// is injected at construction — it could be a local file reader, an in-memory
/// store, or a production secrets manager. Outbound HTTP is provided via an
/// internal [`PostJsonTransport`] and via [`Runtime::create_transport`].
pub struct OmniaRuntime {
    vault: Box<dyn WasiVaultCtx>,
    locker_id: String,
    transport: BoxedPostJsonTransport,
    rt: tokio::runtime::Runtime,
}

impl OmniaRuntime {
    /// Creates an Omnia-backed runtime with the given vault backend and locker ID.
    ///
    /// Returns [`RuntimeError::Other`] if the tokio runtime or the internal HTTP transport cannot be created.
    pub fn new(
        vault: Box<dyn WasiVaultCtx>,
        locker_id: impl Into<String>,
    ) -> Result<Self, RuntimeError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| RuntimeError::Other(format!("failed to create async runtime: {e}")))?;

        let transport: BoxedPostJsonTransport =
            Box::new(OmniaWasiHttpPostJson::new().map_err(|e| {
                RuntimeError::Other(format!("failed to create HTTP transport: {e}"))
            })?);

        Ok(Self {
            vault,
            locker_id: locker_id.into(),
            transport,
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

    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &Value,
    ) -> Result<Vec<u8>, TransportError> {
        self.transport
            .post_json(url, headers, body)
    }

    fn create_transport(&self) -> Result<BoxedPostJsonTransport, RuntimeError> {
        let t = OmniaWasiHttpPostJson::new()
            .map_err(|e| RuntimeError::Other(format!("failed to create transport: {e}")))?;
        Ok(Box::new(t))
    }
}

#[cfg(test)]
mod tests {
    use aria_core::{
        runtime::{Runtime, RuntimeError},
        test_support::named_temp_file_with_writeln,
    };

    use super::OmniaRuntime;
    use crate::infrastructure::adapters::runtime::{
        OmniaWasiVaultAnthropicLocal, ANTHROPIC_VAULT_LOCKER_ID,
    };

    #[test]
    fn get_secret_unknown_name_returns_not_found() -> Result<(), std::io::Error> {
        let tmp = named_temp_file_with_writeln("k")?;
        let vault = Box::new(OmniaWasiVaultAnthropicLocal::new(Some(
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
