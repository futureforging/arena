use aria_core::environment::{Environment, LoggingLevel};

pub struct WasiEnvironment;

impl Environment for WasiEnvironment {
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
