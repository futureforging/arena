//! HTTP arena stub: scripted knock-knock audience and PSI peer (aligned with `aria-core` games).

pub mod psi_peer;

/// TCP port the arena-stub HTTP server binds to on **`127.0.0.1`**.
pub const ARENA_STUB_LISTEN_PORT: u16 = 3000;

/// Full URL for **`POST /message`** (same host/port as the server). README examples should match; see test `arena_stub_message_url_matches_listen_port`.
pub const ARENA_STUB_MESSAGE_URL: &str = "http://127.0.0.1:3000/message";

/// Full URL for **`POST /reset`** (clears scripted peer state). See test `arena_stub_reset_url_matches_listen_port`.
pub const ARENA_STUB_RESET_URL: &str = "http://127.0.0.1:3000/reset";

/// When the teller sends this exact line, the scripted knock-knock sequence restarts from step 0 (same line as turn 1 of a new game).
pub const INVITATION_RESTART_MESSAGE: &str = "Would you like to hear a knock knock joke?";

/// Which game the stub is currently playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameKind {
    KnockKnock,
    Psi,
    Unknown,
}

/// Detects game kind from the first agent message.
pub fn detect_game(message: &str) -> GameKind {
    let lower = message.to_lowercase();
    if lower.contains("private set intersection") || lower.contains("psi") {
        GameKind::Psi
    } else {
        // Default to knock-knock for backward compatibility
        GameKind::KnockKnock
    }
}

/// Combined state: game kind + step counter.
pub struct PeerState {
    pub game: GameKind,
    pub step: u8,
}

impl Default for PeerState {
    fn default() -> Self {
        Self::new()
    }
}

impl PeerState {
    pub fn new() -> Self {
        Self {
            game: GameKind::Unknown,
            step: 0,
        }
    }
}

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

/// Unified turn processor: detects game on first message, then delegates.
pub fn process_turn(state: &mut PeerState, message: &str) -> String {
    if state.game == GameKind::Unknown {
        state.game = detect_game(message);
    }

    if state.game == GameKind::KnockKnock && message == INVITATION_RESTART_MESSAGE {
        state.step = 0;
    }

    let current = state.step;
    state.step = current.saturating_add(1);

    match state.game {
        GameKind::KnockKnock => audience_reply(current, message),
        GameKind::Psi => psi_peer::psi_peer_reply(current, message),
        GameKind::Unknown => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use aria_core::games::sha256_hex;

    use super::{
        audience_reply, detect_game, parse_setup_word, process_audience_turn, process_turn,
        GameKind, PeerState, ARENA_STUB_LISTEN_PORT, ARENA_STUB_MESSAGE_URL, ARENA_STUB_RESET_URL,
        INVITATION_RESTART_MESSAGE,
    };

    #[test]
    fn arena_stub_message_url_matches_listen_port() {
        assert_eq!(
            ARENA_STUB_MESSAGE_URL,
            format!("http://127.0.0.1:{}/message", ARENA_STUB_LISTEN_PORT)
        );
    }

    #[test]
    fn arena_stub_reset_url_matches_listen_port() {
        assert_eq!(
            ARENA_STUB_RESET_URL,
            format!("http://127.0.0.1:{}/reset", ARENA_STUB_LISTEN_PORT)
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

    #[test]
    fn detect_game_psi() {
        assert_eq!(detect_game("Would you like to play Private Set Intersection?"), GameKind::Psi);
    }

    #[test]
    fn detect_game_knock_knock() {
        assert_eq!(detect_game("Would you like to hear a knock knock joke?"), GameKind::KnockKnock);
    }

    #[test]
    fn process_turn_psi_full_exchange() {
        let mut state = PeerState::new();
        let r0 = process_turn(&mut state, "Would you like to play Private Set Intersection?");
        assert_eq!(r0.as_str(), "yes");
        assert_eq!(state.game, GameKind::Psi);

        let r1 = process_turn(&mut state, "I propose we use SHA-256 Hash Intersection.");
        assert_eq!(r1.as_str(), "Agreed. Send your hashes.");

        let h_a = sha256_hex("a");
        let h_m = sha256_hex("m");
        let hashes = format!(r#"["{h_a}","{h_m}"]"#);
        let r2 = process_turn(&mut state, &hashes);
        let parsed: Vec<String> = serde_json::from_str(&r2).expect("JSON");
        assert!(parsed.contains(&"a".to_string()));
        assert!(parsed.contains(&"m".to_string()));

        let r3 = process_turn(&mut state, "Result: correct");
        assert_eq!(r3.as_str(), "Goodbye.");

        let r4 = process_turn(&mut state, "Thanks");
        assert_eq!(r4.as_str(), "");
    }
}
