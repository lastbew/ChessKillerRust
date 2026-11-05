//! game.rs - helpers that decide whose turn it is and how long to think.
//!
//! We do NOT reimplement chess rules: Stockfish applies the move list for us
//! (`position startpos moves ...`). We only need to know *whose turn* it is,
//! which is simply the parity of the number of plies played.

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn name(self) -> &'static str {
        match self {
            Color::White => "White",
            Color::Black => "Black",
        }
    }
}

/// A Lichess `color` field ("white" / "black") -> `Color`.
#[allow(dead_code)]
pub fn color_from_str(text: &str) -> Option<Color> {
    match text {
        "white" => Some(Color::White),
        "black" => Some(Color::Black),
        _ => None,
    }
}

/// Whose turn it is, from the number of plies played from the standard start.
pub fn side_to_move(ply_count: usize) -> Color {
    if ply_count % 2 == 0 {
        Color::White
    } else {
        Color::Black
    }
}

pub fn ply_count(moves: &str) -> usize {
    moves.split_whitespace().count()
}

/// Work out our colour from a `gameFull` event, handling computer opponents.
///
/// A computer player has no `id` (it looks like `{"aiLevel": 4}`), so we use
/// `.get("id")` everywhere instead of indexing `["id"]`.
pub fn detect_my_color(game_full: &Value, my_id: &str) -> Color {
    let white_id = game_full.get("white").and_then(|w| w.get("id")).and_then(Value::as_str);
    let black_id = game_full.get("black").and_then(|b| b.get("id")).and_then(Value::as_str);

    if white_id == Some(my_id) {
        return Color::White;
    }
    if black_id == Some(my_id) {
        return Color::Black;
    }

    // Our id wasn't found: trust whichever side actually has an id (the human).
    match (white_id.is_some(), black_id.is_some()) {
        (true, false) => return Color::White,
        (false, true) => return Color::Black,
        _ => {}
    }

    // Last resort: a side with an AI level is the opponent.
    if game_full.get("white").and_then(|w| w.get("aiLevel")).is_some() {
        Color::Black
    } else {
        Color::White
    }
}

/// Pick how long to think (ms) from the clock, capped by `cap_ms`.
pub fn think_time(state: &Value, side: Color, cap_ms: u64) -> u64 {
    let (time_key, inc_key) = match side {
        Color::White => ("wtime", "winc"),
        Color::Black => ("btime", "binc"),
    };
    let remaining = match state.get(time_key).and_then(Value::as_u64) {
        Some(value) => value,
        None => return cap_ms,
    };
    let increment = state.get(inc_key).and_then(Value::as_u64).unwrap_or(0);

    let mut budget = remaining as f64 / 30.0 + increment as f64 * 0.8;
    budget = budget.max(120.0).min(cap_ms as f64);
    budget = budget.min((remaining as f64 * 0.2).max(120.0));
    budget as u64
}
