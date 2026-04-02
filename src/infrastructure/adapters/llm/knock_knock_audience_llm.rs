use std::cell::RefCell;

use crate::core::{
    llm::{ChatMessage, Llm, LlmCompletion},
    session::ASSISTANT_ROLE,
};

const KNOCK_KNOCK_YES: &str = "yes";

const KNOCK_KNOCK_WHOS_THERE: &str = "Who's there?";

const KNOCK_KNOCK_HAHA: &str = "haha";

/// Returns the setup word from a teller line (first non-empty line, first word, without trailing ASCII punctuation).
pub fn parse_setup_word(text: &str) -> Option<String> {
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

/// Stateful stub `Llm` for the knock-knock **audience** (canned lines in [`Llm::complete`]); instantiate for the **Peer** participant alongside the joke-telling **Agent**.
#[derive(Debug)]
pub struct KnockKnockAudienceLlm {
    step: RefCell<u8>,
}

impl KnockKnockAudienceLlm {
    /// Creates an audience LLM at the first scripted reply (`yes` to the invitation).
    pub fn new() -> Self {
        Self {
            step: RefCell::new(0),
        }
    }
}

impl Default for KnockKnockAudienceLlm {
    fn default() -> Self {
        Self::new()
    }
}

impl Llm for KnockKnockAudienceLlm {
    fn complete(&self, _system: Option<&str>, messages: &[ChatMessage]) -> LlmCompletion {
        let mut step = self
            .step
            .borrow_mut();
        let current = *step;
        *step = current.saturating_add(1);

        let reply = match current {
            0 => KNOCK_KNOCK_YES.to_string(),
            1 => KNOCK_KNOCK_WHOS_THERE.to_string(),
            2 => {
                let last_teller = messages
                    .iter()
                    .rev()
                    .find(|m| m.role == ASSISTANT_ROLE);
                let text = last_teller
                    .map(|m| {
                        m.content
                            .as_str()
                    })
                    .unwrap_or("");
                let word = parse_setup_word(text).unwrap_or_else(|| {
                    eprintln!("Could not parse setup name from teller reply: {text}");
                    std::process::exit(1);
                });
                format!("{word} who?")
            },
            3 => KNOCK_KNOCK_HAHA.to_string(),
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
    use super::{parse_setup_word, KnockKnockAudienceLlm};
    use crate::core::{
        llm::{ChatMessage, Llm},
        session::{ASSISTANT_ROLE, USER_ROLE},
    };

    #[test]
    fn complete_first_turn_is_yes() {
        let llm = KnockKnockAudienceLlm::new();
        let messages = [ChatMessage {
            role: ASSISTANT_ROLE.to_string(),
            content: "Would you like to hear a knock knock joke?".to_string(),
        }];
        let out = llm
            .complete(None, &messages)
            .reply;
        assert_eq!(out, "yes");
    }

    #[test]
    fn complete_second_turn_is_whos_there() {
        let llm = KnockKnockAudienceLlm::new();
        let _ = llm.complete(
            None,
            &[ChatMessage {
                role: ASSISTANT_ROLE.to_string(),
                content: "Invitation?".to_string(),
            }],
        );
        let messages = [
            ChatMessage {
                role: ASSISTANT_ROLE.to_string(),
                content: "Invitation?".to_string(),
            },
            ChatMessage {
                role: USER_ROLE.to_string(),
                content: "yes".to_string(),
            },
            ChatMessage {
                role: ASSISTANT_ROLE.to_string(),
                content: "Knock knock.".to_string(),
            },
        ];
        let out = llm
            .complete(None, &messages)
            .reply;
        assert_eq!(out, "Who's there?");
    }

    #[test]
    fn complete_third_turn_is_setup_who() {
        let llm = KnockKnockAudienceLlm::new();
        let _ = llm.complete(
            None,
            &[ChatMessage {
                role: ASSISTANT_ROLE.to_string(),
                content: "x".to_string(),
            }],
        );
        let _ = llm.complete(
            None,
            &[
                ChatMessage {
                    role: ASSISTANT_ROLE.to_string(),
                    content: "x".to_string(),
                },
                ChatMessage {
                    role: USER_ROLE.to_string(),
                    content: "yes".to_string(),
                },
                ChatMessage {
                    role: ASSISTANT_ROLE.to_string(),
                    content: "Knock knock.".to_string(),
                },
            ],
        );
        let messages = [
            ChatMessage {
                role: ASSISTANT_ROLE.to_string(),
                content: "x".to_string(),
            },
            ChatMessage {
                role: USER_ROLE.to_string(),
                content: "yes".to_string(),
            },
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
    fn complete_fourth_turn_is_haha() {
        let llm = KnockKnockAudienceLlm::new();
        let _ = llm.complete(
            None,
            &[ChatMessage {
                role: ASSISTANT_ROLE.to_string(),
                content: "i".to_string(),
            }],
        );
        let _ = llm.complete(
            None,
            &[
                ChatMessage {
                    role: ASSISTANT_ROLE.to_string(),
                    content: "i".to_string(),
                },
                ChatMessage {
                    role: USER_ROLE.to_string(),
                    content: "yes".to_string(),
                },
                ChatMessage {
                    role: ASSISTANT_ROLE.to_string(),
                    content: "Knock knock.".to_string(),
                },
            ],
        );
        let _ = llm.complete(
            None,
            &[
                ChatMessage {
                    role: ASSISTANT_ROLE.to_string(),
                    content: "i".to_string(),
                },
                ChatMessage {
                    role: USER_ROLE.to_string(),
                    content: "yes".to_string(),
                },
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
                        content: "i".to_string(),
                    },
                    ChatMessage {
                        role: USER_ROLE.to_string(),
                        content: "yes".to_string(),
                    },
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
                        content: "Punchline.".to_string(),
                    },
                ],
            )
            .reply;
        assert_eq!(out, "haha");
    }

    #[test]
    fn parse_setup_single_word_no_punctuation() {
        assert_eq!(parse_setup_word("Boo").as_deref(), Some("Boo"));
    }

    #[test]
    fn parse_setup_strips_trailing_period() {
        assert_eq!(parse_setup_word("Boo.").as_deref(), Some("Boo"));
    }

    #[test]
    fn parse_setup_first_line_first_word() {
        assert_eq!(parse_setup_word("Lettuce\n").as_deref(), Some("Lettuce"));
    }

    #[test]
    fn parse_setup_skips_leading_blank_lines() {
        assert_eq!(parse_setup_word("\n\nOrange.").as_deref(), Some("Orange"));
    }

    #[test]
    fn parse_setup_empty_is_none() {
        assert_eq!(parse_setup_word(""), None);
        assert_eq!(parse_setup_word("   \n  "), None);
    }
}
