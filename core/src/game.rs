/// Configuration for an arena challenge — everything the agent needs to play.
pub struct Challenge {
    /// System prompt that configures the agent for this game.
    pub system_prompt: String,

    /// Optional private data the agent receives at the start
    /// (e.g. "Your wealth is $5M" for Yao's Millionaire).
    /// When present, this is appended to the system prompt.
    pub private_context: Option<String>,

    /// The first message the agent receives to kick off the exchange
    /// (e.g. a short greeting from the peer). In the real arena this would
    /// come from the operator or the peer; here the game provides it
    /// so the agent can initiate.
    pub opening_message: String,
}

/// An arena game that an agent can be instructed to play.
pub trait Game {
    /// Returns the challenge configuration for this game.
    fn challenge(&self) -> Challenge;

    /// Returns `true` when the game is over.
    ///
    /// `turn` is the number of completed peer exchanges (starts at 0 before the first arena round-trip).
    /// `last_peer_reply` is the most recent message received from the peer.
    fn is_complete(&self, turn: usize, last_peer_reply: &str) -> bool;
}
