//! Load the Anthropic API key from a local file for use by adapters (e.g. Claude LLM).
//!
//! With `path` [`None`], reads `anthropic_api_key.txt` at the root of this repository
//! (parent of the `tools/` crate), using compile-time [`CARGO_MANIFEST_DIR`] for that crate.

use std::path::{Path, PathBuf};

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

fn project_root_default_key_file() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tools crate manifest must have a parent (project root)")
        .join("anthropic_api_key.txt")
}

/// Reads and returns the trimmed API key from `path`, or from the project-root default file when `path` is [`None`].
pub fn anthropic_api_key_from_local_file(
    path: Option<&Path>,
) -> Result<String, AnthropicApiKeyError> {
    let path = match path {
        Some(p) => p.to_path_buf(),
        None => project_root_default_key_file(),
    };
    let raw = std::fs::read_to_string(&path).map_err(|source| AnthropicApiKeyError::Read {
        path: path.clone(),
        source,
    })?;
    let key = raw.trim();
    if key.is_empty() {
        return Err(AnthropicApiKeyError::Empty(path));
    }
    Ok(key.to_string())
}
