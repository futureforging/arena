use sha2::{Digest, Sha256};

/// Hardcoded peer letter set for the PSI stub (intersection is computed against this set).
pub const PSI_PEER_SET: &[char] = &['a', 'c', 'e', 'g', 'i', 'k', 'm', 'o', 'q', 's'];

/// Reply when the peer accepts the SHA-256 strategy (must stay in sync with `play_wasi` in `secure-agent`).
pub const PSI_PEER_AGREED_MESSAGE: &str = "Agreed. Send your hashes.";

pub fn psi_peer_reply(step: u8, _agent_message: &str) -> String {
    match step {
        0 => "yes".to_string(),
        1 => {
            println!(
                "[PRIVATE — local only, not sent on the wire] arena-stub peer private letter set: {:?}",
                PSI_PEER_SET
            );
            PSI_PEER_AGREED_MESSAGE.to_string()
        },
        2 => compute_intersection(_agent_message),
        3 => "Goodbye.".to_string(),
        _ => String::new(),
    }
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn compute_intersection(agent_hashes_message: &str) -> String {
    let agent_hashes: Vec<String> =
        extract_json_string_array(agent_hashes_message).unwrap_or_default();

    let mut intersection: Vec<char> = Vec::new();
    for &letter in PSI_PEER_SET {
        let hash = sha256_hex(&letter.to_string());
        if agent_hashes
            .iter()
            .any(|h| h == &hash)
        {
            intersection.push(letter);
        }
    }

    let as_strings: Vec<String> = intersection
        .iter()
        .map(|c| c.to_string())
        .collect();
    serde_json::to_string(&as_strings).unwrap_or_else(|_| "[]".to_string())
}

/// Extracts the first JSON array of strings from a message.
/// Handles cases where the LLM wraps the array in prose.
pub fn extract_json_string_array(text: &str) -> Option<Vec<String>> {
    let start = text.find('[')?;
    let end = text.rfind(']')?;
    if end <= start {
        return None;
    }
    let slice = &text[start..=end];
    serde_json::from_str(slice).ok()
}

#[cfg(test)]
mod tests {
    use verity_core::games::sha256_hex;

    use super::{extract_json_string_array, psi_peer_reply, PSI_PEER_SET};

    #[test]
    fn psi_peer_step_0_says_yes() {
        assert_eq!(psi_peer_reply(0, "any").as_str(), "yes");
    }

    #[test]
    fn psi_peer_step_1_agrees() {
        assert_eq!(psi_peer_reply(1, "proposal").as_str(), "Agreed. Send your hashes.");
    }

    #[test]
    fn psi_peer_step_2_computes_intersection() {
        let h_a = sha256_hex("a");
        let h_c = sha256_hex("c");
        let msg = format!(r#"Here are my hashes: ["{h_a}","{h_c}"]"#);
        let reply = psi_peer_reply(2, &msg);
        assert!(reply.contains('a'));
        assert!(reply.contains('c'));
        let parsed: Vec<String> = serde_json::from_str(&reply).expect("peer returns JSON array");
        assert!(parsed.contains(&"a".to_string()));
        assert!(parsed.contains(&"c".to_string()));
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn psi_peer_step_3_says_goodbye() {
        assert_eq!(psi_peer_reply(3, "Result: correct").as_str(), "Goodbye.");
    }

    #[test]
    fn psi_peer_step_4_returns_empty() {
        assert_eq!(psi_peer_reply(4, "x").as_str(), "");
    }

    #[test]
    fn extract_json_string_array_with_preamble() {
        let inner = PSI_PEER_SET
            .iter()
            .map(|c| format!("\"{}\"", sha256_hex(&c.to_string())))
            .collect::<Vec<_>>()
            .join(",");
        let text = format!("Prefix text [{inner}] trailing");
        let arr = extract_json_string_array(&text).expect("array");
        assert_eq!(arr.len(), PSI_PEER_SET.len());
    }
}
