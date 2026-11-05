//! config.rs - load settings from `.env` / the environment.

use std::env;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub token: String,
    pub challenge_username: String,
    pub dry_run: bool,
    pub move_time_ms: u64,
    pub clock_minutes: u32,
    pub clock_increment: u32,
    pub stockfish_path: Option<PathBuf>,
}

fn text(name: &str, default: &str) -> String {
    env::var(name)
        .unwrap_or_else(|_| default.to_string())
        .trim()
        .to_string()
}

pub fn load() -> Result<Config> {
    // Load .env if present; ignore if there is none.
    let _ = dotenvy::dotenv();

    let dry_run = matches!(
        text("DRY_RUN", "1").to_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    );
    let stockfish_env = text("STOCKFISH_PATH", "");

    Ok(Config {
        token: env::var("LICHESS_TOKEN").unwrap_or_default().trim().to_string(),
        challenge_username: text("CHALLENGE_USERNAME", ""),
        dry_run,
        move_time_ms: text("MOVE_TIME_MS", "800").parse().unwrap_or(800),
        clock_minutes: text("CLOCK_MINUTES", "3").parse().unwrap_or(3),
        clock_increment: text("CLOCK_INCREMENT", "2").parse().unwrap_or(2),
        stockfish_path: if stockfish_env.is_empty() {
            None
        } else {
            Some(PathBuf::from(stockfish_env))
        },
    })
}

pub fn require_token(config: &Config) -> Result<()> {
    if config.token.is_empty() {
        bail!("No LICHESS_TOKEN. Copy .env.example to .env and add your token (scope board:play).");
    }
    Ok(())
}

/// Find Stockfish: the explicit `STOCKFISH_PATH`, else the first `.exe` in `./engines/`.
pub fn find_stockfish(config: &Config) -> Result<PathBuf> {
    if let Some(path) = &config.stockfish_path {
        if path.exists() {
            return Ok(path.clone());
        }
        bail!("STOCKFISH_PATH points to {}, which does not exist", path.display());
    }

    let engines = Path::new("engines");
    if engines.is_dir() {
        for entry in std::fs::read_dir(engines)? {
            let path = entry?.path();
            let is_exe = path
                .extension()
                .map(|ext| ext.to_string_lossy().eq_ignore_ascii_case("exe"))
                .unwrap_or(false);
            if is_exe {
                return Ok(path);
            }
        }
    }

    bail!("Could not find Stockfish. Put stockfish.exe in ./engines/ or set STOCKFISH_PATH in .env")
}
