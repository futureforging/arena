use std::cell::RefCell;

use crate::core::{
    llm::{ChatMessage, Llm, LlmCompletion},
    session::ASSISTANT_ROLE,
};

const KNOCK_KNOCK_WHOS_THERE: &str = "Who's there?";

const KNOCK_KNOCK_HAHA: &str = "haha";

/// Returns the setup word from the assistant’s message after "Who's there?" (first non-empty line, first word, without trailing ASCII punctuation).
pub fn parse_setup_from_assistant_reply(text: &str) -> Option<String> {
    let first_line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?;
    let first_word = first_line
        .split_whitespace()
        .next()?;
    let word = first_word.trim_end_matches(|c: char| c.is_ascii_punctuation());
    if word.is_empty() {
        None
    } else {
        Some(word.to_string())
    }
}

/// Stateful stub `Llm` that plays the user side of the scripted knock-knock exchange (hardcoded lines in [`Llm::complete`]).
#[derive(Debug)]
pub struct KnockKnockUserLlm {
    step: RefCell<u8>,
}

impl KnockKnockUserLlm {
    /// Creates a user LLM at the first scripted reply (`Who's there?`).
    pub fn new() -> Self {
        Self {
            step: RefCell::new(0),
        }
    }
}

impl Default for KnockKnockUserLlm {
    fn default() -> Self {
        Self::new()
    }
}

impl Llm for KnockKnockUserLlm {
    fn complete(&self, _system: Option<&str>, messages: &[ChatMessage]) -> LlmCompletion {
        let mut step = self
            .step
            .borrow_mut();
        let current = *step;
        *step = current.saturating_add(1);

        let reply = match current {
            0 => KNOCK_KNOCK_WHOS_THERE.to_string(),
            1 => {
                let last_assistant = messages
                    .iter()
                    .rev()
                    .find(|m| m.role == ASSISTANT_ROLE);
                let text = last_assistant
                    .map(|m| {
                        m.content
                            .as_str()
                    })
                    .unwrap_or("");
                let word = parse_setup_from_assistant_reply(text).unwrap_or_else(|| {
                    eprintln!("Could not parse setup name from assistant reply: {text}");
                    std::process::exit(1);
                });
                format!("{word} who?")
            },
            2 => KNOCK_KNOCK_HAHA.to_string(),
            _ => String::new(),
        };

        LlmCompletion {
            reply,
            request_body_json: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_setup_from_assistant_reply, KnockKnockUserLlm};
    use crate::core::{
        llm::{ChatMessage, Llm},
        session::{ASSISTANT_ROLE, USER_ROLE},
    };

    #[test]
    fn complete_first_turn_is_whos_there() {
        let llm = KnockKnockUserLlm::new();
        let messages = [ChatMessage {
            role: ASSISTANT_ROLE.to_string(),
            content: "Knock knock.".to_string(),
        }];
        let out = llm
            .complete(None, &messages)
            .reply;
        assert_eq!(out, "Who's there?");
    }

    #[test]
    fn complete_second_turn_is_setup_who() {
        let llm = KnockKnockUserLlm::new();
        let _ = llm.complete(
            None,
            &[ChatMessage {
                role: ASSISTANT_ROLE.to_string(),
                content: "Knock knock.".to_string(),
            }],
        );
        let messages = [
            ChatMessage {
                role: ASSISTANT_ROLE.to_string(),
                content: "Knock knock.".to_string(),
            },
            ChatMessage {
                role: USER_ROLE.to_string(),
                content: "Who's there?".to_string(),
            },
            ChatMessage {
                role: ASSISTANT_ROLE.to_string(),
                content: "Boo.".to_string(),
            },
        ];
        let out = llm
            .complete(None, &messages)
            .reply;
        assert_eq!(out, "Boo who?");
    }

    #[test]
    fn complete_third_turn_is_haha() {
        let llm = KnockKnockUserLlm::new();
        let _ = llm.complete(
            None,
            &[ChatMessage {
                role: ASSISTANT_ROLE.to_string(),
                content: "Knock knock.".to_string(),
            }],
        );
        let _ = llm.complete(
            None,
            &[
                ChatMessage {
                    role: ASSISTANT_ROLE.to_string(),
                    content: "Knock knock.".to_string(),
                },
                ChatMessage {
                    role: USER_ROLE.to_string(),
                    content: "Who's there?".to_string(),
                },
                ChatMessage {
                    role: ASSISTANT_ROLE.to_string(),
                    content: "Boo.".to_string(),
                },
            ],
        );
        let out = llm
            .complete(
                None,
                &[
                    ChatMessage {
                        role: ASSISTANT_ROLE.to_string(),
                        content: "Knock knock.".to_string(),
                    },
                    ChatMessage {
                        role: USER_ROLE.to_string(),
                        content: "Who's there?".to_string(),
                    },
                    ChatMessage {
                        role: ASSISTANT_ROLE.to_string(),
                        content: "Boo.".to_string(),
                    },
                    ChatMessage {
                        role: USER_ROLE.to_string(),
                        content: "Boo who?".to_string(),
                    },
                    ChatMessage {
                        role: ASSISTANT_ROLE.to_string(),
                        content: "Boo.".to_string(),
                    },
                ],
            )
            .reply;
        assert_eq!(out, "haha");
    }

    #[test]
    fn parse_setup_single_word_no_punctuation() {
        assert_eq!(parse_setup_from_assistant_reply("Boo").as_deref(), Some("Boo"));
    }

    #[test]
    fn parse_setup_strips_trailing_period() {
        assert_eq!(parse_setup_from_assistant_reply("Boo.").as_deref(), Some("Boo"));
    }

    #[test]
    fn parse_setup_first_line_first_word() {
        assert_eq!(parse_setup_from_assistant_reply("Lettuce\n").as_deref(), Some("Lettuce"));
    }

    #[test]
    fn parse_setup_skips_leading_blank_lines() {
        assert_eq!(parse_setup_from_assistant_reply("\n\nOrange.").as_deref(), Some("Orange"));
    }

    #[test]
    fn parse_setup_empty_is_none() {
        assert_eq!(parse_setup_from_assistant_reply(""), None);
        assert_eq!(parse_setup_from_assistant_reply("   \n  "), None);
    }
}
