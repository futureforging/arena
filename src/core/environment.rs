/// Configured verbosity for an environment (how much log output is allowed).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoggingLevel {
    None,
    Standard,
    Verbose,
}

/// Level of an individual log line passed to [`Environment::log`](Environment::log).
/// Only [`Standard`](LogMessageLevel::Standard) and [`Verbose`](LogMessageLevel::Verbose) are valid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogMessageLevel {
    Standard,
    Verbose,
}

/// Returns whether a message at `msg` may be emitted for the given environment `env`.
///
/// Hierarchy: [`LoggingLevel::None`](LoggingLevel::None) drops all logs;
/// [`LoggingLevel::Standard`](LoggingLevel::Standard) allows only [`LogMessageLevel::Standard`](LogMessageLevel::Standard);
/// [`LoggingLevel::Verbose`](LoggingLevel::Verbose) allows both message levels.
pub fn log_message_is_allowed(env: LoggingLevel, msg: LogMessageLevel) -> bool {
    match env {
        LoggingLevel::None => false,
        LoggingLevel::Standard => matches!(msg, LogMessageLevel::Standard),
        LoggingLevel::Verbose => true,
    }
}

/// Routes agent output to the host environment (e.g. shell).
pub trait Environment {
    /// Prints `s` to the environment in the appropriate way.
    fn print(&self, s: &str);

    /// Current logging level for this environment.
    fn logging_level(&self) -> LoggingLevel;

    /// Logs `message` at `level` when allowed for [`logging_level`](Environment::logging_level), then calls [`emit_log`](Environment::emit_log).
    fn log(&self, message: &str, level: LogMessageLevel) {
        if !log_message_is_allowed(self.logging_level(), level) {
            return;
        }
        self.emit_log(message);
    }

    /// Writes a log line after [`log`](Environment::log) has applied the level filter.
    fn emit_log(&self, message: &str);
}

#[cfg(test)]
mod tests {
    use super::{log_message_is_allowed, Environment, LogMessageLevel, LoggingLevel};
    use crate::test_support::InMemoryEnvironment;

    #[test]
    fn log_message_is_allowed_respects_hierarchy() {
        assert!(!log_message_is_allowed(LoggingLevel::None, LogMessageLevel::Standard));
        assert!(!log_message_is_allowed(LoggingLevel::None, LogMessageLevel::Verbose));
        assert!(log_message_is_allowed(LoggingLevel::Standard, LogMessageLevel::Standard));
        assert!(!log_message_is_allowed(LoggingLevel::Standard, LogMessageLevel::Verbose));
        assert!(log_message_is_allowed(LoggingLevel::Verbose, LogMessageLevel::Standard));
        assert!(log_message_is_allowed(LoggingLevel::Verbose, LogMessageLevel::Verbose));
    }

    #[test]
    fn environment_log_default_delegates_to_emit_log_when_allowed() {
        let env = InMemoryEnvironment::new(LoggingLevel::Standard);
        env.log("a", LogMessageLevel::Standard);
        env.log("b", LogMessageLevel::Verbose);
        assert_eq!(env.logged_lines(), vec![String::from("a")]);

        let env_v = InMemoryEnvironment::new(LoggingLevel::Verbose);
        env_v.log("s2", LogMessageLevel::Standard);
        env_v.log("v2", LogMessageLevel::Verbose);
        assert_eq!(env_v.logged_lines(), vec![String::from("s2"), String::from("v2")]);

        let env_n = InMemoryEnvironment::new(LoggingLevel::None);
        env_n.log("x", LogMessageLevel::Standard);
        env_n.log("y", LogMessageLevel::Verbose);
        assert!(env_n
            .logged_lines()
            .is_empty());
    }
}
