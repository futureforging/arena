//! Adapters that implement [`verity_core`](verity_core) ports against concrete host environments
//! (CLI/stdio today; HTTP, web UI, etc. as they're added).
//!
//! Adapters depend only on `verity-core` and may be consumed by any binary or guest crate that
//! needs a ready-made implementation of a port.

pub mod cli_environment;

pub use cli_environment::CliEnvironment;
