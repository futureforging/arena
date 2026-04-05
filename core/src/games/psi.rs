use sha2::{Digest, Sha256};

use crate::game::{Challenge, Game};

const MAX_TURNS: usize = 15;

const BASE_SYSTEM_PROMPT: &str =
    "You are playing Private Set Intersection with your peer. Be concise at every step.";

const PSI_SESSION_PROMPT: &str = r#"You are playing a Private Set Intersection (PSI) game. You have a private set of letters. Your goal is to find which letters you and your peer have in common WITHOUT revealing letters that are not in the intersection.

Turn order:
1) The peer opens with a brief greeting (e.g. "Hello."). Ask if they would like to play Private Set Intersection (one short sentence ending with a question mark).
2) When the peer says "yes", propose the strategy: "I propose we use SHA-256 Hash Intersection. I will send you SHA-256 hashes of each item in my set. You hash your own items and compare. Then send me back the plaintext intersection." Wait for acceptance.
3) When the peer accepts (says "agreed" or similar), send your hashed items as a JSON array on a single line, like: ["aef3...", "b2c4...", ...]. Use the exact hashes from your private data below. Do not add any other text on that line.
4) The peer will respond with the plaintext intersection as a JSON array, like: ["a", "c", "f"]. Verify every item in their claimed intersection appears in YOUR private set. If all items are in your set, respond with "Result: correct". If any item is NOT in your set, respond with "Result: incorrect".
5) After reporting the result, say "Goodbye." on its own line.

IMPORTANT: When sending hashes in step 3, send ONLY the JSON array on a single line. No preamble, no explanation. Use the exact hex hashes provided in your private data."#;

const OPENING_MESSAGE: &str = "Hello.";

/// Compute the SHA-256 hex digest of `input`.
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// Private Set Intersection arena game using a hashed-set exchange (see session prompt).
pub struct PsiGame {
    private_set: Vec<char>,
}

impl PsiGame {
    /// Creates a new PSI game with the given private set of letters.
    pub fn new(private_set: Vec<char>) -> Self {
        Self {
            private_set,
        }
    }

    /// This game's private letters (for local logging only; never sent verbatim to the peer).
    pub fn private_set(&self) -> &[char] {
        &self.private_set
    }

    /// Builds the private context string: the agent's letters and their SHA-256 hashes.
    fn build_private_context(&self) -> String {
        let items: Vec<String> = self
            .private_set
            .iter()
            .map(|c| {
                let hash = sha256_hex(&c.to_string());
                format!("  '{c}' -> {hash}")
            })
            .collect();

        let hash_json: Vec<String> = self
            .private_set
            .iter()
            .map(|c| format!("\"{}\"", sha256_hex(&c.to_string())))
            .collect();

        format!(
            "Your private set: {:?}\n\nYour items with their SHA-256 hashes:\n{}\n\nWhen you send hashes, send exactly this JSON array:\n[{}]",
            self.private_set,
            items.join("\n"),
            hash_json.join(", "),
        )
    }
}

impl Game for PsiGame {
    fn challenge(&self) -> Challenge {
        let system_prompt = format!("{BASE_SYSTEM_PROMPT}\n\n{PSI_SESSION_PROMPT}");
        Challenge {
            system_prompt,
            private_context: Some(self.build_private_context()),
            opening_message: OPENING_MESSAGE.to_string(),
        }
    }

    fn is_complete(&self, turn: usize, last_peer_reply: &str) -> bool {
        turn >= MAX_TURNS
            || last_peer_reply
                .to_lowercase()
                .contains("goodbye")
    }
}

#[cfg(test)]
mod tests {
    use super::{sha256_hex, PsiGame};
    use crate::game::Game;

    #[test]
    fn challenge_has_private_context_with_hashes() {
        let game = PsiGame::new(vec!['a', 'b']);
        let c = game.challenge();
        assert!(c
            .private_context
            .is_some());
        let ctx = c
            .private_context
            .unwrap();
        assert!(ctx.contains("Your private set:"));
        assert!(ctx.contains("ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"));
    }

    #[test]
    fn is_complete_on_goodbye() {
        let game = PsiGame::new(vec!['a']);
        assert!(game.is_complete(1, "Goodbye."));
    }

    #[test]
    fn is_complete_on_max_turns() {
        let game = PsiGame::new(vec!['a']);
        assert!(game.is_complete(15, "still here"));
    }

    #[test]
    fn is_complete_false_for_normal_turn() {
        let game = PsiGame::new(vec!['a']);
        assert!(!game.is_complete(1, "yes"));
    }

    #[test]
    fn sha256_hex_deterministic() {
        assert_eq!(
            sha256_hex("a"),
            "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
        );
    }
}
