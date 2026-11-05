//! menu.rs - the interactive CLI menu.

use std::io::{self, Write};

use anyhow::Result;

use crate::config::Config;
use crate::engine::Engine;
use crate::lichess::LichessClient;

fn read_line() -> Option<String> {
    let mut buffer = String::new();
    match io::stdin().read_line(&mut buffer) {
        Ok(0) => None, // EOF
        Ok(_) => Some(buffer.trim().to_string()),
        Err(_) => None,
    }
}

fn prompt(label: &str) -> Option<String> {
    print!("{label}");
    io::stdout().flush().ok();
    read_line()
}

pub fn run_menu(
    client: &LichessClient,
    engine: &mut Engine,
    config: &mut Config,
    my_id: &str,
    username: &str,
) -> Result<()> {
    loop {
        println!();
        println!(
            "Logged in as {username} | DRY_RUN={} | think {}ms | clock {}+{}",
            if config.dry_run { 1 } else { 0 },
            config.move_time_ms,
            config.clock_minutes,
            config.clock_increment
        );
        if !config.challenge_username.is_empty() {
            println!("Challenge target: {}", config.challenge_username);
        }
        println!("------------------------------------------------------------");
        println!(" 1) Auto-play (wait for a game, play, repeat)");
        println!(" 2) Challenge a user, then auto-play");
        println!(
            " 3) Toggle dry-run (now {})",
            if config.dry_run { "ON" } else { "OFF" }
        );
        println!(" 4) Set think time (now {} ms)", config.move_time_ms);
        println!(" 5) Quit");

        let choice = match prompt("> ") {
            Some(value) => value,
            None => return Ok(()),
        };

        match choice.as_str() {
            "1" => {
                return crate::run_continuous(client, engine, config, my_id);
            }
            "2" => {
                if let Some(name) = prompt("Username to challenge: ") {
                    if !name.is_empty() {
                        config.challenge_username = name;
                    }
                }
                return crate::run_continuous(client, engine, config, my_id);
            }
            "3" => {
                config.dry_run = !config.dry_run;
            }
            "4" => {
                if let Some(text) = prompt("Think time in ms (>=50): ") {
                    if let Ok(value) = text.parse::<u64>() {
                        if value >= 50 {
                            config.move_time_ms = value;
                        }
                    }
                }
            }
            "5" | "q" | "quit" | "exit" => return Ok(()),
            _ => println!("Please choose 1-5."),
        }
    }
}
