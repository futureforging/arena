use verity_core::environment::{Environment, LoggingLevel};

/// [`Environment`] adapter that prints user-facing output to stdout and log lines to stderr.
///
/// Suitable for CLI binaries and any host that exposes the standard streams (including the WASI
/// guest, which inherits stdio from the runtime).
pub struct CliEnvironment;

impl Environment for CliEnvironment {
    fn print(&self, s: &str) {
        println!("{s}");
    }

    fn logging_level(&self) -> LoggingLevel {
        LoggingLevel::Standard
    }

    fn emit_log(&self, message: &str) {
        eprintln!("{message}");
    }
}
