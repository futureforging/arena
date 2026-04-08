use crate::{
    agent::Agent,
    environment::Environment,
    game::Game,
    llm::Llm,
    session::{ReceiveMessageError, Session, StartSessionError, ASSISTANT_ROLE, USER_ROLE},
    tool::ToolError,
};

/// Error from [`play_game`].
#[derive(Debug, Eq, PartialEq)]
pub enum PlayGameError {
    /// Failed to start the agent session.
    SessionStart(StartSessionError),
    /// The agent failed to process a message.
    AgentReceive(ReceiveMessageError),
    /// The `"arena"` tool failed or returned an unusable response.
    Tool(ToolError),
}

/// Plays a [`Game`] to completion using the given agent.
///
/// Arena messages are sent via the agent's [`crate::agent::Agent::tools`] registry under the name `"arena"`.
///
/// 1. Configures the agent with the game's challenge (system prompt + private context).
/// 2. Sends the opening message to the agent to get the first reply.
/// 3. Loops: sends agent's reply through the arena tool, gets peer's response,
///    feeds it back to the agent, until the game says it's complete.
/// 4. Stops the session and returns the number of completed turns.
pub fn play_game<E: Environment, L: Llm>(
    agent: &mut Agent<E, L>,
    game: &dyn Game,
) -> Result<usize, PlayGameError> {
    let challenge = game.challenge();

    let system_prompt = match challenge.private_context {
        Some(ref ctx) => format!("{}\n\n{ctx}", challenge.system_prompt),
        None => challenge.system_prompt,
    };

    agent
        .start_session(Session::new(system_prompt), ASSISTANT_ROLE, USER_ROLE)
        .map_err(PlayGameError::SessionStart)?;

    let mut agent_reply = agent
        .receive_message(&challenge.opening_message)
        .map_err(PlayGameError::AgentReceive)?;

    let mut turn = 0;
    loop {
        let input = serde_json::json!({ "message": agent_reply });
        let result = agent
            .tools
            .execute("arena", &input)
            .map_err(PlayGameError::Tool)?;
        let peer_reply = result["reply"]
            .as_str()
            .ok_or_else(|| {
                PlayGameError::Tool(ToolError::ExecutionFailed(
                    "arena tool returned no 'reply' field".to_string(),
                ))
            })?
            .to_string();

        turn += 1;

        if game.is_complete(turn, &peer_reply) {
            break;
        }

        agent_reply = agent
            .receive_message(&peer_reply)
            .map_err(PlayGameError::AgentReceive)?;
    }

    let _ = agent.stop_session();
    Ok(turn)
}

#[cfg(test)]
mod tests {
    use super::{play_game, PlayGameError};
    use crate::{
        agent::Agent,
        environment::LoggingLevel,
        session::{Session, StartSessionError, ASSISTANT_ROLE, USER_ROLE},
        test_support::{
            agent_with_stub, FailingArenaTool, InMemoryEnvironment, StubArenaTool, StubGame,
            StubLlm,
        },
        tool::{ToolError, ToolRegistry},
    };

    #[test]
    fn play_game_returns_session_start_when_session_already_active() {
        let mut agent = agent_with_stub();
        agent
            .start_session(Session::new("x"), ASSISTANT_ROLE, USER_ROLE)
            .unwrap();
        let game = StubGame {
            max_turns: 5,
        };
        let err = play_game(&mut agent, &game).unwrap_err();
        assert_eq!(err, PlayGameError::SessionStart(StartSessionError::AlreadyActive));
    }

    #[test]
    fn play_game_completes_when_arena_returns_empty_after_scripted_replies() {
        let arena_tool = StubArenaTool::new(vec![String::from("r1"), String::from("r2")]);
        let mut agent = Agent {
            name: String::from("a"),
            environment: InMemoryEnvironment::new(LoggingLevel::Standard),
            llm: StubLlm::default(),
            tools: ToolRegistry::new(vec![Box::new(arena_tool)]),
            active_session: None,
        };
        let game = StubGame {
            max_turns: 100,
        };
        let turns = play_game(&mut agent, &game).unwrap();
        assert_eq!(turns, 3);
    }

    #[test]
    fn play_game_stops_when_turn_reaches_max_even_if_peer_reply_non_empty() {
        let arena_tool =
            StubArenaTool::new(vec![String::from("a"), String::from("b"), String::from("c")]);
        let mut agent = Agent {
            name: String::from("a"),
            environment: InMemoryEnvironment::new(LoggingLevel::Standard),
            llm: StubLlm::default(),
            tools: ToolRegistry::new(vec![Box::new(arena_tool)]),
            active_session: None,
        };
        let game = StubGame {
            max_turns: 2,
        };
        let turns = play_game(&mut agent, &game).unwrap();
        assert_eq!(turns, 2);
    }

    #[test]
    fn play_game_propagates_arena_error() {
        let mut agent = Agent {
            name: String::from("a"),
            environment: InMemoryEnvironment::new(LoggingLevel::Standard),
            llm: StubLlm::default(),
            tools: ToolRegistry::new(vec![Box::new(FailingArenaTool)]),
            active_session: None,
        };
        let game = StubGame {
            max_turns: 5,
        };
        let err = play_game(&mut agent, &game).unwrap_err();
        assert_eq!(err, PlayGameError::Tool(ToolError::ExecutionFailed(String::from("boom"))));
    }
}
