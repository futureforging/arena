mod application;
mod core;
mod infrastructure;

#[cfg(test)]
mod test_support;

pub use core::{
    agent::Agent,
    arena::{Arena, ArenaError},
    environment::{log_message_is_allowed, Environment, LogMessageLevel, LoggingLevel},
    llm::{ChatMessage, Llm, LlmCompletion},
    runtime::{Runtime, RuntimeError, ANTHROPIC_API_KEY_SECRET},
    session::{
        merge_system_prompts, ActiveSession, ReceiveMessageError, Session, StartSessionError,
        ASSISTANT_ROLE, USER_ROLE,
    },
    transport::{
        BoxedPostJsonTransport, IntoBoxedPostJsonTransport, PostJsonTransport, TransportError,
    },
};

pub use application::factories::create_agent::create_agent;
pub use infrastructure::adapters::{
    environment::ShellEnvironment,
    llm::{ClaudeLlm, DummyLlm, KnockKnockAudienceLlm},
    ArenaHttpClient, OmniaRuntime, OmniaWasiHttpPostJson, OmniaWasiVaultAnthropicLocal,
    SecureAgent, ANTHROPIC_VAULT_LOCKER_ID, ANTHROPIC_VAULT_SECRET_ID,
};
