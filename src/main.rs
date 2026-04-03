use aria_core::{game_loop::play_game, games::KnockKnockGame};
use aria_poc_2::{
    ArenaHttpClient, LoggingLevel, OmniaRuntime, OmniaWasiVaultAnthropicLocal, Runtime,
    SecureAgent, ShellEnvironment, ANTHROPIC_VAULT_LOCKER_ID,
};

fn main() {
    // Runtime (vault for API key + outbound HTTP)
    let vault = Box::new(OmniaWasiVaultAnthropicLocal::new(None));
    let runtime = match OmniaRuntime::new(vault, ANTHROPIC_VAULT_LOCKER_ID) {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to create OmniaRuntime: {e:?}");
            std::process::exit(1);
        },
    };

    // Arena client (talks to arena-stub on localhost:3000)
    let arena_transport = match runtime.create_transport() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to create arena transport: {e:?}");
            std::process::exit(1);
        },
    };
    let arena = ArenaHttpClient::new("http://127.0.0.1:3000", arena_transport);

    // SecureAgent
    let mut agent = match SecureAgent::new(
        runtime,
        None,
        ShellEnvironment {
            logging_level: LoggingLevel::None,
        },
    ) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to initialize SecureAgent: {e:?}");
            std::process::exit(1);
        },
    };

    let game = KnockKnockGame;
    match play_game(&mut *agent, &arena, &game) {
        Ok(turns) => {
            eprintln!("Game complete after {turns} turns.");
        },
        Err(e) => {
            eprintln!("Game failed: {e:?}");
            std::process::exit(1);
        },
    }
}
