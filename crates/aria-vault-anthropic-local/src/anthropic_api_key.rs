//! Load the Anthropic API key from a local file for use by adapters (e.g. Claude LLM).
//!
//! With `path` [`None`], reads `anthropic_api_key.txt` at the repository root (parent of the
//! `crates/` directory), using compile-time [`CARGO_MANIFEST_DIR`] for this crate.

use std::path::Path;

pub use crate::key_source::AnthropicApiKeyError;
use crate::key_source::{default_repo_root_key_file, read_anthropic_key_strict};

/// Reads and returns the trimmed API key from `path`, or from the project-root default file when `path` is [`None`].
pub fn anthropic_api_key_from_local_file(
    path: Option<&Path>,
) -> Result<String, AnthropicApiKeyError> {
    let path = match path {
        Some(p) => p.to_path_buf(),
        None => default_repo_root_key_file(),
    };
    read_anthropic_key_strict(&path)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::key_source::read_anthropic_key_strict;

    #[test]
    fn from_local_file_matches_key_source_strict() {
        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
        writeln!(tmp, "  same-key  ").expect("write");
        let path = tmp.path();
        assert_eq!(
            anthropic_api_key_from_local_file(Some(path)).expect("from_local_file"),
            read_anthropic_key_strict(path).expect("strict")
        );
    }
}
