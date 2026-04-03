use crate::game::{Challenge, Game};

/// Base system instructions for the knock-knock teller
/// (merged with the session prompt on every completion).
const BASE_SYSTEM_PROMPT: &str =
    "You are telling a knock-knock joke to your peer. Be concise at every step.";

/// Session-scoped instructions for the joke teller.
const TELLER_SESSION_PROMPT: &str = r#"You are running a fixed knock-knock joke exchange. You are the joke teller; your peer is the audience.

Turn order:
1) The peer opens with a brief greeting (e.g. "Hello."). Reply only by asking if they would like to hear a knock knock joke (one short sentence ending with a question mark).
2) When the peer says "yes", reply with only this on the first line: Knock knock.
3) When the peer says "Who's there?", put the setup name as exactly one word on the first line of your message (no leading label or punctuation before that word). The peer will say "{word} who?".
4) When the peer says "{word} who?" using that setup word, give a short punchline.
5) When the peer says "haha", reply with one brief parting pleasantry only. The scripted exchange ends there.

Be concise at every step."#;

/// Synthetic peer greeting to kick off the exchange.
const OPENING_MESSAGE: &str = "Hello.";

/// Maximum turns before the game is considered done regardless.
const MAX_TURNS: usize = 10;

/// The knock-knock joke exchange as an arena game.
pub struct KnockKnockGame;

impl Game for KnockKnockGame {
    fn challenge(&self) -> Challenge {
        let system_prompt = format!("{BASE_SYSTEM_PROMPT}\n\n{TELLER_SESSION_PROMPT}");
        Challenge {
            system_prompt,
            private_context: None,
            opening_message: OPENING_MESSAGE.to_string(),
        }
    }

    fn is_complete(&self, turn: usize, last_peer_reply: &str) -> bool {
        turn >= MAX_TURNS || last_peer_reply.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::KnockKnockGame;
    use crate::game::Game;

    #[test]
    fn knock_knock_challenge_has_non_empty_system_prompt_and_expected_opening() {
        let game = KnockKnockGame;
        let c = game.challenge();
        assert!(!c
            .system_prompt
            .is_empty());
        assert!(c
            .system_prompt
            .contains("knock-knock"));
        assert_eq!(c.opening_message, "Hello.");
        assert!(c
            .private_context
            .is_none());
    }

    #[test]
    fn is_complete_false_for_early_turn_with_non_empty_peer() {
        let game = KnockKnockGame;
        assert!(!game.is_complete(1, "yes"));
    }

    #[test]
    fn is_complete_true_when_peer_reply_empty() {
        let game = KnockKnockGame;
        assert!(game.is_complete(1, ""));
    }

    #[test]
    fn is_complete_true_at_max_turns() {
        let game = KnockKnockGame;
        assert!(game.is_complete(10, "still talking"));
    }
}
