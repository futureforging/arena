use std::ops::{Deref, DerefMut};

use crate::{
    core::agent::Agent,
    infrastructure::adapters::{environment::ShellEnvironment, llm::ClaudeLlm},
};

const DISPLAY_NAME: &str = "SecureAgent";

/// Shell-backed environment, Anthropic [`ClaudeLlm`], fixed display name **SecureAgent**.
pub struct SecureAgent(Agent<ShellEnvironment, ClaudeLlm>);

impl SecureAgent {
    /// Assembles the core [`Agent`] with [`ShellEnvironment`] and [`ClaudeLlm::new`].
    pub fn new(
        api_key: impl Into<String>,
        system_prompt: Option<String>,
        environment: ShellEnvironment,
    ) -> Self {
        Self(Agent {
            name: DISPLAY_NAME.to_string(),
            environment,
            llm: ClaudeLlm::new(api_key, system_prompt),
            active_session: None,
        })
    }
}

impl Deref for SecureAgent {
    type Target = Agent<ShellEnvironment, ClaudeLlm>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SecureAgent {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
