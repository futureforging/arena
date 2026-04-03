//! Knock-knock audience state machine for the HTTP arena stub (script aligned with the knock-knock game in `aria-core`).

/// TCP port the arena-stub HTTP server binds to on **`127.0.0.1`**.
pub const ARENA_STUB_LISTEN_PORT: u16 = 3000;

/// Full URL for **`POST /message`** (same host/port as the server). README examples should match; see test `arena_stub_message_url_matches_listen_port`.
pub const ARENA_STUB_MESSAGE_URL: &str = "http://127.0.0.1:3000/message";

/// When the teller sends this exact line, the scripted audience sequence restarts from step 0 (same line as turn 1 of a new game).
pub const INVITATION_RESTART_MESSAGE: &str = "Would you like to hear a knock knock joke?";

/// First non-empty line, first word, without trailing ASCII punctuation (matches `KnockKnockAudienceLlm`).
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

pub fn audience_reply(current: u8, teller_message: &str) -> String {
    match current {
        0 => "yes".to_string(),
        1 => "Who's there?".to_string(),
        2 => parse_setup_word(teller_message)
            .map(|word| format!("{word} who?"))
            .unwrap_or_default(),
        3 => "haha".to_string(),
        _ => String::new(),
    }
}

/// Applies one teller line: optional reset, advances step, returns audience line.
pub fn process_audience_turn(step: &mut u8, message: &str) -> String {
    if message == INVITATION_RESTART_MESSAGE {
        *step = 0;
    }
    let current = *step;
    *step = current.saturating_add(1);
    audience_reply(current, message)
}

#[cfg(test)]
mod tests {
    use super::{
        audience_reply, parse_setup_word, process_audience_turn, ARENA_STUB_LISTEN_PORT,
        ARENA_STUB_MESSAGE_URL, INVITATION_RESTART_MESSAGE,
    };

    #[test]
    fn arena_stub_message_url_matches_listen_port() {
        assert_eq!(
            ARENA_STUB_MESSAGE_URL,
            format!("http://127.0.0.1:{}/message", ARENA_STUB_LISTEN_PORT)
        );
    }

    #[test]
    fn parse_setup_strips_trailing_period() {
        assert_eq!(parse_setup_word("Boo.").as_deref(), Some("Boo"));
    }

    #[test]
    fn audience_first_step_is_yes() {
        assert_eq!(audience_reply(0, "any").as_str(), "yes");
    }

    #[test]
    fn audience_second_step_is_whos_there() {
        assert_eq!(audience_reply(1, "Knock knock.").as_str(), "Who's there?");
    }

    #[test]
    fn audience_third_step_is_setup_who() {
        assert_eq!(audience_reply(2, "Boo").as_str(), "Boo who?");
    }

    #[test]
    fn audience_fourth_step_is_haha() {
        assert_eq!(audience_reply(3, "punch").as_str(), "haha");
    }

    #[test]
    fn full_scripted_exchange_steps_match() {
        let mut step = 0u8;
        let r0 = process_audience_turn(&mut step, INVITATION_RESTART_MESSAGE);
        let r1 = process_audience_turn(&mut step, "Knock knock.");
        let r2 = process_audience_turn(&mut step, "Boo");
        let r3 = process_audience_turn(&mut step, "line");
        let r4 = process_audience_turn(&mut step, "more");
        assert_eq!(
            (r0.as_str(), r1.as_str(), r2.as_str(), r3.as_str(), r4.as_str(), step),
            ("yes", "Who's there?", "Boo who?", "haha", "", 5u8)
        );
    }

    #[test]
    fn invitation_resets_mid_game() {
        let mut step = 3u8;
        let reply = process_audience_turn(&mut step, INVITATION_RESTART_MESSAGE);
        assert_eq!((reply.as_str(), step), ("yes", 1u8));
    }

    #[test]
    fn invitation_at_step_zero_still_yes() {
        let mut step = 0u8;
        let reply = process_audience_turn(&mut step, INVITATION_RESTART_MESSAGE);
        assert_eq!(reply.as_str(), "yes");
    }
}
