//! watcher.rs - describe a game's mode and wait for a new game to appear.

use std::collections::HashSet;
use std::time::Duration;

use anyhow::Result;
use serde_json::Value;

use crate::lichess::LichessClient;

#[derive(Debug, Clone)]
pub struct GameMode {
    pub game_id: String,
    pub speed: String,
    pub rated: bool,
    pub variant: String,
    pub color: String,
    pub opponent: String,
    #[allow(dead_code)]
    pub is_computer: bool,
    pub is_standard: bool,
}

impl GameMode {
    pub fn summary(&self) -> String {
        let rated = if self.rated { "rated" } else { "unrated" };
        format!(
            "{} | {} | you are {} | vs {} | {}",
            capitalize(&self.speed),
            rated,
            capitalize(&self.color),
            self.opponent,
            self.variant
        )
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

pub fn describe_mode(game: &Value) -> GameMode {
    let opponent = game.get("opponent").cloned().unwrap_or(Value::Null);
    let ai_level = opponent.get("ai").and_then(Value::as_u64);
    let raw_name = opponent
        .get("username")
        .and_then(Value::as_str)
        .or_else(|| opponent.get("id").and_then(Value::as_str))
        .unwrap_or("opponent")
        .to_string();
    let (display_name, is_computer) = match ai_level {
        Some(level) => (format!("Stockfish level {level}"), true),
        None => (raw_name, false),
    };
    let variant = game
        .get("variant")
        .and_then(|v| v.get("key"))
        .and_then(Value::as_str)
        .unwrap_or("standard")
        .to_string();

    GameMode {
        game_id: game
            .get("gameId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        speed: game.get("speed").and_then(Value::as_str).unwrap_or("?").to_string(),
        rated: game.get("rated").and_then(Value::as_bool).unwrap_or(false),
        is_standard: variant == "standard",
        variant,
        color: game.get("color").and_then(Value::as_str).unwrap_or("?").to_string(),
        opponent: display_name,
        is_computer,
    }
}

/// Poll the account until a game id we have not seen before appears.
pub fn wait_for_new_game(
    client: &LichessClient,
    seen: &mut HashSet<String>,
) -> Result<Value> {
    loop {
        for game in client.get_ongoing_games()? {
            let id = game
                .get("gameId")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if !id.is_empty() && !seen.contains(&id) {
                return Ok(game);
            }
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}
