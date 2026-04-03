//! Local file–backed [`crate::core::runtime::Runtime`] for the Anthropic API key.

use std::path::{Path, PathBuf};

use crate::core::runtime::{Runtime, RuntimeError};

/// Default filename for the Anthropic API key at the repository root.
const DEFAULT_KEY_FILE_NAME: &str = "anthropic_api_key.txt";

/// Secret name expected for the Anthropic API key.
pub const ANTHROPIC_API_KEY_SECRET: &str = "anthropic_api_key";

/// Failed to read a non-empty API key from the given file.
#[derive(Debug)]
pub enum AnthropicApiKeyError {
    /// Could not read the file.
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    /// File was empty or only whitespace after trimming.
    Empty(PathBuf),
}

impl std::fmt::Display for AnthropicApiKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read {
                path,
                source,
            } => write!(f, "{}: {source}", path.display()),
            Self::Empty(path) => write!(f, "{}: file is empty or only whitespace", path.display()),
        }
    }
}

impl std::error::Error for AnthropicApiKeyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read {
                source,
                ..
            } => Some(source),
            Self::Empty(_) => None,
        }
    }
}

/// Default path: `anthropic_api_key.txt` at the repository root (relative to this package’s [`CARGO_MANIFEST_DIR`]).
fn default_repo_root_key_file() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_KEY_FILE_NAME)
}

/// Reads and returns the trimmed non-empty API key from `path` (strict: missing or empty is an error).
fn read_anthropic_key_strict(path: &Path) -> Result<String, AnthropicApiKeyError> {
    let path_buf = path.to_path_buf();
    let raw = std::fs::read_to_string(path).map_err(|source| AnthropicApiKeyError::Read {
        path: path_buf.clone(),
        source,
    })?;
    let key = raw.trim();
    if key.is_empty() {
        return Err(AnthropicApiKeyError::Empty(path_buf));
    }
    Ok(key.to_string())
}

/// [`Runtime`] backed by local file reads. Resolves `get_secret("anthropic_api_key")` by reading the configured path or the default repo-root key file.
pub struct LocalFileRuntime {
    key_file: Option<PathBuf>,
}

impl LocalFileRuntime {
    /// Creates a runtime that reads secrets from local files.
    /// Pass [`None`] to use the default repo-root key file location.
    pub fn new(key_file: Option<PathBuf>) -> Self {
        Self {
            key_file,
        }
    }

    fn anthropic_api_key_from_configured_file(&self) -> Result<String, AnthropicApiKeyError> {
        let path = match &self.key_file {
            Some(p) => p.clone(),
            None => default_repo_root_key_file(),
        };
        read_anthropic_key_strict(&path)
    }
}

impl Runtime for LocalFileRuntime {
    fn get_secret(&self, name: &str) -> Result<String, RuntimeError> {
        match name {
            ANTHROPIC_API_KEY_SECRET => self
                .anthropic_api_key_from_configured_file()
                .map_err(|e| RuntimeError::Other(e.to_string())),
            other => Err(RuntimeError::NotFound(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        read_anthropic_key_strict, AnthropicApiKeyError, LocalFileRuntime, ANTHROPIC_API_KEY_SECRET,
    };
    use crate::{
        core::runtime::{Runtime, RuntimeError},
        test_support::named_temp_file_with_writeln,
    };

    #[test]
    fn read_anthropic_key_strict_trims_non_empty_file() -> Result<(), std::io::Error> {
        let tmp = named_temp_file_with_writeln("  key-from-test  ")?;
        let path = tmp.path();
        let key =
            read_anthropic_key_strict(path).map_err(|e| std::io::Error::other(format!("{e}")))?;
        assert_eq!(key, "key-from-test");
        Ok(())
    }

    #[test]
    fn get_secret_anthropic_key_reads_temp_file() -> Result<(), std::io::Error> {
        let tmp = named_temp_file_with_writeln("  test-api-key-value  ")?;
        let path = tmp
            .path()
            .to_path_buf();
        let rt = LocalFileRuntime::new(Some(path));
        assert_eq!(rt.get_secret(ANTHROPIC_API_KEY_SECRET), Ok("test-api-key-value".to_string()));
        Ok(())
    }

    #[test]
    fn get_secret_unknown_name_returns_not_found() {
        let rt = LocalFileRuntime::new(None);
        assert_eq!(
            rt.get_secret("unknown_secret"),
            Err(RuntimeError::NotFound("unknown_secret".to_string()))
        );
    }

    #[test]
    fn read_anthropic_key_strict_empty_file_returns_empty_variant() -> Result<(), std::io::Error> {
        let tmp = named_temp_file_with_writeln("   ")?;
        let path = tmp.path();
        let result = read_anthropic_key_strict(path);
        assert!(matches!(result, Err(AnthropicApiKeyError::Empty(_))));
        Ok(())
    }
}
