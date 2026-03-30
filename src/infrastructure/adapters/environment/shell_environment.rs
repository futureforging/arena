use crate::core::environment::Environment;

/// Sends output to the process shell via `println!`.
#[derive(Clone, Copy, Debug)]
pub struct ShellEnvironment;

impl Environment for ShellEnvironment {
    fn print(&self, s: &str) {
        println!("{s}");
    }
}
