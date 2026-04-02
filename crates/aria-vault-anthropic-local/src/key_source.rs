//! Shared path and read semantics for the anemic Anthropic local-file vault (one file, one secret).

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

/// Default path: `anthropic_api_key.txt` at the repository root (using this crate’s location under `crates/`).
pub fn default_repo_root_key_file() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate manifest must live under crates/")
        .parent()
        .expect("crates directory must have a parent (repository root)")
        .join("anthropic_api_key.txt")
}

/// Reads and returns the trimmed non-empty API key from `path` (strict: missing or empty is an error).
pub fn read_anthropic_key_strict(path: &Path) -> Result<String, AnthropicApiKeyError> {
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

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn strict_read_matches_public_api_for_temp_file() {
        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
        writeln!(tmp, "  key-from-test  ").expect("write");
        let path = tmp.path();

        let from_strict = read_anthropic_key_strict(path).expect("strict");
        assert_eq!(from_strict, "key-from-test");
    }
}
