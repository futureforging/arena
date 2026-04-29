use sha2::{Digest, Sha256};

use crate::game::{Challenge, Game};

const MAX_TURNS: usize = 15;

const BASE_SYSTEM_PROMPT: &str =
    "You are playing Private Set Intersection with your peer. Be concise at every step.";

const PSI_SECOND_MOVER_SESSION_PROMPT: &str = r#"You are playing a Private Set Intersection (PSI) game. You have a private set of numbers. Your goal is to find which numbers you and your peer have in common WITHOUT revealing numbers that are not in the intersection.

Turn order:
1) The peer opens with a brief greeting (e.g. "Hello."). Ask if they would like to play Private Set Intersection (one short sentence ending with a question mark).
2) When the peer says "yes", propose the strategy: "I propose we use SHA-256 Hash Intersection. I will send you SHA-256 hashes of each item in my set. You hash your own items and compare. Then send me back the plaintext intersection." Wait for acceptance.
3) When the peer accepts (says "agreed" or similar), send your hashed items as a JSON array on a single line, like: ["aef3...", "b2c4...", ...]. Use the exact hashes from your private data below. Do not add any other text on that line.
4) The peer will respond with the plaintext intersection as a JSON array, like: [263, 804, 213]. Verify every item in their claimed intersection appears in YOUR private set. If all items are in your set, respond with "Result: correct". If any item is NOT in your set, respond with "Result: incorrect".
5) After reporting the result, say "Goodbye." on its own line.

IMPORTANT: When sending hashes in step 3, send ONLY the JSON array on a single line. No preamble, no explanation. Use the exact hex hashes provided in your private data."#;

const PSI_FIRST_MOVER_SESSION_PROMPT: &str = r#"You are playing a Private Set Intersection (PSI) game. You have a private set of numbers. Your goal is to find which numbers you and your peer have in common WITHOUT revealing numbers that are not in the intersection.

You opened the conversation with "Hello." and your peer is now responding. Turn order from here:
1) The peer will ask if you want to play Private Set Intersection. Reply with a single word: "yes".
2) The peer will propose using SHA-256 hash intersection (e.g. "I propose we use SHA-256 Hash Intersection..."). Reply with a single word: "agreed".
3) The peer will send their hashed items as a JSON array on a single line, like: ["aef3...", "b2c4...", ...]. Compute the plaintext intersection: for each of THEIR hashes, check whether it appears in YOUR list of hashes (provided in your private data below). For every match, take the corresponding plaintext number from YOUR private data. Reply with a single-line JSON array of those plaintext numbers, like: [263, 804, 213]. No preamble, no explanation.
4) The peer will reply with "Result: correct" (or "Result: incorrect") and "Goodbye." on a separate line. Reply with "Goodbye." on its own line to acknowledge and end the conversation.

IMPORTANT: When sending the intersection in step 3, send ONLY the JSON array on a single line. Use exact numbers from your set."#;

const OPENING_MESSAGE: &str = "Hello.";

/// Whether this PSI game instance plays the first-mover or second-mover script.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    /// Sends `"Hello."` first, then drives the responsive half of the protocol
    /// (says yes to playing, agrees to strategy, computes intersection, says goodbye).
    First,
    /// Waits for the peer's `"Hello."`, then drives the proposing half
    /// (asks to play, proposes strategy, sends hashes, verifies result).
    Second,
}

/// Compute the SHA-256 hex digest of `input`.
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// Private Set Intersection arena game using a hashed-set exchange (see session prompt).
pub struct PsiGame {
    private_set: Vec<u32>,
    role: Role,
}

impl PsiGame {
    /// Creates a new PSI game with the given private numbers and mover role.
    pub fn new(private_set: Vec<u32>, role: Role) -> Self {
        Self {
            private_set,
            role,
        }
    }

    pub fn role(&self) -> Role {
        self.role
    }

    /// This game's private numbers (for local logging only; never sent verbatim to the peer).
    pub fn private_set(&self) -> &[u32] {
        &self.private_set
    }

    /// Compute the intersection of our private set against a list of peer hashes.
    /// Used by `play_psi_wasi` to determine the answer to submit (first-mover path).
    pub fn intersection_against_peer_hashes(&self, peer_hashes: &[String]) -> Vec<u32> {
        self.private_set
            .iter()
            .copied()
            .filter(|n| {
                let h = sha256_hex(&n.to_string());
                peer_hashes
                    .iter()
                    .any(|ph| ph == &h)
            })
            .collect()
    }

    /// Builds the private context string: the agent's numbers and their SHA-256 hashes.
    fn build_private_context(&self) -> String {
        let items: Vec<String> = self
            .private_set
            .iter()
            .map(|n| {
                let hash = sha256_hex(&n.to_string());
                format!("  {n} -> {hash}")
            })
            .collect();

        let hash_json: Vec<String> = self
            .private_set
            .iter()
            .map(|n| format!("\"{}\"", sha256_hex(&n.to_string())))
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
        let session_prompt = match self.role {
            Role::First => PSI_FIRST_MOVER_SESSION_PROMPT,
            Role::Second => PSI_SECOND_MOVER_SESSION_PROMPT,
        };
        let system_prompt = format!("{BASE_SYSTEM_PROMPT}\n\n{session_prompt}");
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
    use super::{sha256_hex, PsiGame, Role};
    use crate::game::Game;

    #[test]
    fn challenge_has_private_context_with_hashes() {
        let game = PsiGame::new(vec![1, 2], Role::Second);
        let c = game.challenge();
        assert!(c
            .private_context
            .is_some());
        let ctx = c
            .private_context
            .unwrap();
        assert!(ctx.contains("Your private set:"));
        assert!(ctx.contains("6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b"));
    }

    #[test]
    fn first_and_second_mover_system_prompts_differ() {
        let first = PsiGame::new(vec![1], Role::First).challenge();
        let second = PsiGame::new(vec![1], Role::Second).challenge();
        assert_ne!(first.system_prompt, second.system_prompt);
        assert!(first
            .system_prompt
            .contains("You opened the conversation"));
        assert!(second
            .system_prompt
            .contains("The peer opens with a brief greeting"));
    }

    #[test]
    fn is_complete_on_goodbye() {
        let game = PsiGame::new(vec![1], Role::Second);
        assert!(game.is_complete(1, "Goodbye."));
    }

    #[test]
    fn is_complete_on_max_turns() {
        let game = PsiGame::new(vec![1], Role::Second);
        assert!(game.is_complete(15, "still here"));
    }

    #[test]
    fn is_complete_false_for_normal_turn() {
        let game = PsiGame::new(vec![1], Role::Second);
        assert!(!game.is_complete(1, "yes"));
    }

    #[test]
    fn sha256_hex_deterministic() {
        assert_eq!(
            sha256_hex("a"),
            "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
        );
    }

    #[test]
    fn intersection_against_peer_hashes_returns_overlap() {
        let game = PsiGame::new(vec![10, 20, 30], Role::Second);
        let peer_hashes = vec![sha256_hex("20"), sha256_hex("30"), sha256_hex("99")];
        let intersection = game.intersection_against_peer_hashes(&peer_hashes);
        assert_eq!(intersection, vec![20, 30]);
    }
}
