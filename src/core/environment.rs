/// Routes agent output to the host environment (e.g. shell).
pub trait Environment {
    /// Prints `s` to the environment in the appropriate way.
    fn print(&self, s: &str);
}
