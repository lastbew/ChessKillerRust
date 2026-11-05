//! main.rs - ChessKiller (Rust): play on Lichess with Stockfish.
//!
//!   cargo run --release              # play a game that is already running
//!   cargo run --release -- --watch   # lobby: auto-play any new game

mod config;
mod engine;
mod game;
mod lichess;
mod menu;
mod watcher;

use std::collections::HashSet;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::config::Config;
use crate::engine::Engine;
use crate::game::{detect_my_color, ply_count, side_to_move, think_time, Color};
use crate::lichess::LichessClient;
use crate::watcher::{describe_mode, wait_for_new_game};

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let watch = args.iter().any(|arg| arg == "--watch");
    let once = args.iter().any(|arg| arg == "--once");

    let mut config = config::load()?;
    config::require_token(&config)?;
    let stockfish = config::find_stockfish(&config)?;

    let client = LichessClient::new(&config.token)?;
    let account = client.get_account()?;
    let my_id = account.get("id").and_then(Value::as_str).unwrap_or("").to_string();
    let username = account
        .get("username")
        .and_then(Value::as_str)
        .unwrap_or(&my_id)
        .to_string();
    println!("Logged in as {username} (id: {my_id})");

    let mut engine = Engine::spawn(&stockfish)?;
    let result = if watch {
        run_continuous(&client, &mut engine, &config, &my_id)
    } else if once {
        run_once(&client, &mut engine, &config, &my_id)
    } else {
        menu::run_menu(&client, &mut engine, &mut config, &my_id, &username)
    };
    engine.quit();
    result
}

fn run_once(
    client: &LichessClient,
    engine: &mut Engine,
    config: &Config,
    my_id: &str,
) -> Result<()> {
    let game_id = match find_game(client, config)? {
        Some(id) => id,
        None => {
            println!(
                "No game to play. Set CHALLENGE_USERNAME, start a game on Lichess, or run with --watch."
            );
            return Ok(());
        }
    };
    println!("Playing game: https://lichess.org/{game_id}");
    if config.dry_run {
        println!("DRY-RUN mode: moves are printed but NOT sent.");
    }
    play_game(client, engine, config, &game_id, my_id)
}

fn find_game(client: &LichessClient, config: &Config) -> Result<Option<String>> {
    if !config.challenge_username.is_empty() {
        println!(
            "Challenging {} (waiting for them to accept)...",
            config.challenge_username
        );
        client.create_challenge(
            &config.challenge_username,
            config.clock_minutes,
            config.clock_increment,
        )?;
        return client.wait_for_game(120);
    }
    Ok(client
        .get_ongoing_games()?
        .into_iter()
        .next()
        .and_then(|game| game.get("gameId").and_then(Value::as_str).map(str::to_string)))
}

/// Continuous mode: play a game, and when it ends, wait for the next one.
pub(crate) fn run_continuous(
    client: &LichessClient,
    engine: &mut Engine,
    config: &Config,
    my_id: &str,
) -> Result<()> {
    let mut seen: HashSet<String> = HashSet::new();

    // Optional: challenge someone first (if CHALLENGE_USERNAME is set), then keep going.
    if !config.challenge_username.is_empty() {
        println!(
            "Challenging {} (waiting for them to accept)...",
            config.challenge_username
        );
        client.create_challenge(
            &config.challenge_username,
            config.clock_minutes,
            config.clock_increment,
        )?;
        match client.wait_for_game(120)? {
            Some(id) => {
                seen.insert(id.clone());
                println!("Playing game: https://lichess.org/{id}");
                if config.dry_run {
                    println!("DRY-RUN mode: moves are printed but NOT sent.");
                }
                if let Err(error) = play_game(client, engine, config, &id, my_id) {
                    eprintln!("Game error: {error:#}");
                }
                println!("Game over.");
            }
            None => println!("No one accepted the challenge."),
        }
    }

    loop {
        println!("Waiting for a new game... (start one on Lichess)");
        let game = wait_for_new_game(client, &mut seen)?;
        let mode = describe_mode(&game);
        seen.insert(mode.game_id.clone());
        println!("Detected: {}", mode.summary());

        if !mode.is_standard {
            println!("Skipping variant '{}' (only standard is supported).", mode.variant);
            continue;
        }

        println!("Playing game: https://lichess.org/{}", mode.game_id);
        if config.dry_run {
            println!("DRY-RUN mode: moves are printed but NOT sent.");
        }
        if let Err(error) = play_game(client, engine, config, &mode.game_id, my_id) {
            eprintln!("Game error: {error:#}");
        }
        println!("Game over. Waiting for the next game...\n");
    }
}

fn play_game(
    client: &LichessClient,
    engine: &mut Engine,
    config: &Config,
    game_id: &str,
    my_id: &str,
) -> Result<()> {
    let mut my_color: Option<Color> = None;
    let mut initial_fen: Option<String> = None;

    for event in client.stream_game(game_id)? {
        match event.get("type").and_then(Value::as_str).unwrap_or("") {
            "gameFull" => {
                let color = detect_my_color(&event, my_id);
                my_color = Some(color);
                initial_fen = event
                    .get("initialFen")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                println!("We are playing as {}.", color.name());
                if let Some(state) = event.get("state") {
                    maybe_move(client, engine, config, state, my_color, &initial_fen, game_id)?;
                }
            }
            "gameState" => {
                maybe_move(client, engine, config, &event, my_color, &initial_fen, game_id)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn maybe_move(
    client: &LichessClient,
    engine: &mut Engine,
    config: &Config,
    state: &Value,
    my_color: Option<Color>,
    initial_fen: &Option<String>,
    game_id: &str,
) -> Result<()> {
    let status = state.get("status").and_then(Value::as_str).unwrap_or("started");
    if status != "started" && status != "created" {
        println!("Game finished (status: {status}).");
        return Ok(());
    }

    let my_color = match my_color {
        Some(color) => color,
        None => return Ok(()),
    };
    let moves = state.get("moves").and_then(Value::as_str).unwrap_or("");
    let side = side_to_move(ply_count(moves));
    if side != my_color {
        println!("{} to move - waiting for opponent.", side.name());
        return Ok(());
    }

    engine.set_position(initial_fen.as_deref(), moves)?;
    let think = think_time(state, side, config.move_time_ms);
    let chosen = match engine.best_move(think)? {
        Some(mv) => mv,
        None => {
            println!("No legal move - game over.");
            return Ok(());
        }
    };
    println!("Our turn. Stockfish plays: {chosen}");

    if config.dry_run {
        println!("  (dry-run: not sending)");
        return Ok(());
    }

    let ok = client
        .make_move(game_id, &chosen)
        .with_context(|| format!("sending move {chosen}"))?;
    if !ok {
        eprintln!("  move {chosen} was rejected");
    }
    Ok(())
}

