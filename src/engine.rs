//! engine.rs - talk to Stockfish over the UCI protocol via stdin/stdout.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use anyhow::{Context, Result};

pub struct Engine {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Engine {
    /// Start Stockfish and complete the UCI handshake.
    pub fn spawn(path: &Path) -> Result<Self> {
        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("failed to start Stockfish at {}", path.display()))?;

        let stdin = child.stdin.take().context("Stockfish had no stdin")?;
        let stdout = BufReader::new(child.stdout.take().context("Stockfish had no stdout")?);
        let mut engine = Engine { child, stdin, stdout };
        engine.send("uci")?;
        engine.read_until(|line| line == "uciok")?;
        Ok(engine)
    }

    fn send(&mut self, command: &str) -> Result<()> {
        writeln!(self.stdin, "{command}")?;
        self.stdin.flush()?;
        Ok(())
    }

    /// Read lines until `done(line)` is true (used for `uciok` / `bestmove`).
    fn read_until<F: Fn(&str) -> bool>(&mut self, done: F) -> Result<Vec<String>> {
        let mut lines = Vec::new();
        let mut buffer = String::new();
        loop {
            buffer.clear();
            let read = self.stdout.read_line(&mut buffer)?;
            if read == 0 {
                break;
            }
            let line = buffer.trim_end().to_string();
            let finished = done(&line);
            lines.push(line);
            if finished {
                break;
            }
        }
        Ok(lines)
    }

    /// Set the position: optional FEN (`None`/`"startpos"` = normal start) plus UCI moves.
    pub fn set_position(&mut self, fen: Option<&str>, moves: &str) -> Result<()> {
        let base = match fen {
            Some(value) if !value.is_empty() && value != "startpos" => format!("fen {value}"),
            _ => "startpos".to_string(),
        };
        let command = if moves.trim().is_empty() {
            format!("position {base}")
        } else {
            format!("position {base} moves {moves}")
        };
        self.send(&command)
    }

    /// Ask for the best move. `None` means there is no legal move (game over).
    pub fn best_move(&mut self, movetime_ms: u64) -> Result<Option<String>> {
        self.send(&format!("go movetime {movetime_ms}"))?;
        let lines = self.read_until(|line| line.starts_with("bestmove"))?;
        let last = match lines.last() {
            Some(line) => line,
            None => return Ok(None),
        };
        let rest = last.strip_prefix("bestmove").unwrap_or("").trim();
        let mv = rest.split_whitespace().next().unwrap_or("");
        if mv.is_empty() || mv == "(none)" || mv == "0000" {
            Ok(None)
        } else {
            Ok(Some(mv.to_string()))
        }
    }

    /// Shut Stockfish down cleanly.
    pub fn quit(mut self) {
        let _ = self.send("quit");
        let _ = self.child.wait();
    }
}
