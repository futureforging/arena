//! Reverse-scan transcript helpers for PSI guess extraction (peer role messages).

use verity_core::llm::ChatMessage;

pub(crate) fn extract_peer_hash_array(
    transcript: &[ChatMessage],
    peer_role: &str,
) -> Option<Vec<String>> {
    for msg in transcript
        .iter()
        .rev()
    {
        if msg.role != peer_role {
            continue;
        }
        let trimmed = msg
            .content
            .trim();
        let Some(start) = trimmed.find('[') else {
            continue;
        };
        let Some(end) = trimmed.rfind(']') else {
            continue;
        };
        if end <= start {
            continue;
        }
        if let Ok(arr) = serde_json::from_str::<Vec<String>>(&trimmed[start..=end]) {
            if arr
                .iter()
                .all(|s| {
                    s.len() == 64
                        && s.chars()
                            .all(|c| c.is_ascii_hexdigit())
                })
            {
                return Some(arr);
            }
        }
    }
    None
}

pub(crate) fn extract_peer_number_array(
    transcript: &[ChatMessage],
    peer_role: &str,
) -> Option<Vec<u32>> {
    for msg in transcript
        .iter()
        .rev()
    {
        if msg.role != peer_role {
            continue;
        }
        let trimmed = msg
            .content
            .trim();
        let Some(start) = trimmed.find('[') else {
            continue;
        };
        let Some(end) = trimmed.rfind(']') else {
            continue;
        };
        if end <= start {
            continue;
        }
        if let Ok(arr) = serde_json::from_str::<Vec<u32>>(&trimmed[start..=end]) {
            return Some(arr);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use verity_core::llm::ChatMessage;

    use super::{extract_peer_hash_array, extract_peer_number_array};

    #[test]
    fn extract_peer_hash_array_skips_non_array_peer_messages() {
        let hash = "0".repeat(64);
        let arr_str = format!("[\"{hash}\"]");
        let transcript = vec![
            ChatMessage {
                role: "user".into(),
                content: arr_str.clone(),
            },
            ChatMessage {
                role: "assistant".into(),
                content: "[1, 2, 3]".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: "Result: correct\n\nGoodbye.".into(),
            },
            ChatMessage {
                role: "assistant".into(),
                content: "Goodbye.".into(),
            },
        ];
        let extracted = extract_peer_hash_array(&transcript, "user");
        assert_eq!(extracted, Some(vec![hash]));
    }

    #[test]
    fn extract_peer_number_array_skips_non_array_peer_messages() {
        let transcript = vec![
            ChatMessage {
                role: "user".into(),
                content: "[10, 20, 30]".into(),
            },
            ChatMessage {
                role: "assistant".into(),
                content: "ok".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: "Goodbye.".into(),
            },
            ChatMessage {
                role: "assistant".into(),
                content: "Goodbye.".into(),
            },
        ];
        let extracted = extract_peer_number_array(&transcript, "user");
        assert_eq!(extracted, Some(vec![10, 20, 30]));
    }
}
