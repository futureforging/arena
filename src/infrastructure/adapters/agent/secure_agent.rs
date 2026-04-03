use std::ops::{Deref, DerefMut};

use crate::{
    core::{
        agent::Agent,
        runtime::{Runtime, RuntimeError, ANTHROPIC_API_KEY_SECRET},
        transport::PostJsonTransport,
    },
    infrastructure::adapters::{environment::ShellEnvironment, llm::ClaudeLlm},
};

const DISPLAY_NAME: &str = "SecureAgent";

/// Shell-backed environment, Anthropic [`ClaudeLlm`], fixed display name **SecureAgent**. The API key is resolved only in [`SecureAgent::new`] via a [`Runtime`] (see [`Runtime::get_secret`]); the core [`Agent`] does not carry a runtime. Outbound JSON POSTs use the injected [`PostJsonTransport`].
pub struct SecureAgent(Agent<ShellEnvironment, ClaudeLlm>);

impl SecureAgent {
    /// Assembles the core [`Agent`] with [`ShellEnvironment`] and [`ClaudeLlm::new`], resolving the API key via [`Runtime::get_secret`] and not retaining the runtime on the agent.
    pub fn new<R: Runtime, T: PostJsonTransport + Send + Sync + 'static>(
        runtime: R,
        transport: T,
        system_prompt: Option<String>,
        environment: ShellEnvironment,
    ) -> Result<Self, RuntimeError> {
        let api_key = runtime.get_secret(ANTHROPIC_API_KEY_SECRET)?;
        Ok(Self(Agent {
            name: DISPLAY_NAME.to_string(),
            environment,
            llm: ClaudeLlm::new(api_key, system_prompt, transport),
            active_session: None,
        }))
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
