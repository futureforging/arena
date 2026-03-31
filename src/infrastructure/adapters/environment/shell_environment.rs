use crate::core::environment::{Environment, LoggingLevel};

/// Sends output to the process shell via `println!` for [`Environment::print`](Environment::print)
/// and `eprintln!` for [`Environment::emit_log`](Environment::emit_log).
#[derive(Clone, Copy, Debug)]
pub struct ShellEnvironment {
    pub logging_level: LoggingLevel,
}

impl Environment for ShellEnvironment {
    fn print(&self, s: &str) {
        println!("{s}");
    }

    fn logging_level(&self) -> LoggingLevel {
        self.logging_level
    }

    fn emit_log(&self, message: &str) {
        eprintln!("{message}");
    }
}
