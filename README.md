# ChessKiller (Rust)

Plays chess on [Lichess](https://lichess.org) using **Stockfish**.
This is the Rust version of the ChessKiller bot.

## Run
Open a terminal in this folder:

```powershell
.\chesskiller-rust.exe            # shows the menu
.\chesskiller-rust.exe --watch    # skip the menu: auto-play continuously
.\chesskiller-rust.exe --once     # play a single game, then exit
```

**Continuous by default:** when a game finishes it **waits for the next one** and
keeps playing — it does not force-close after a match. Press Ctrl-C to stop.

## Menu
```
Logged in as <you> | DRY_RUN=0 | think 800ms | clock 3+2
------------------------------------------------------------
 1) Auto-play (wait for a game, play, repeat)
 2) Challenge a user, then auto-play
 3) Toggle dry-run
 4) Set think time
 5) Quit
```
Menu settings (dry-run, think time) apply for the current session.

## Configure (.env)
- `LICHESS_TOKEN` — your Lichess token (starts with `lip_`, scope `board:play`)
- `DRY_RUN` — `0` plays for real, `1` only prints the moves it would play
- `CHALLENGE_USERNAME` — optional: who to challenge (blank = resume a game)
- `MOVE_TIME_MS`, `CLOCK_MINUTES`, `CLOCK_INCREMENT`

`DRY_RUN=1` is a safe way to test without sending moves.

`.env` is gitignored — only `.env.example` is committed. If a token is ever
leaked, revoke it at https://lichess.org/account/oauth/token and generate a new one.

## Stockfish
`stockfish.exe` is in `engines/`. To use a different one, put it there, or set
`STOCKFISH_PATH` in `.env`.

## Rebuild (only if you change the code)
This build uses the GNU target + MinGW. Extract a MinGW-w64 toolchain into `mingw\`,
then create your local linker config from the tracked template (add your own path):

```powershell
Copy-Item .cargo\config.toml.example .cargo\config.toml   # then edit the linker path
$env:PATH = "$PWD\mingw\mingw64\bin;$env:PATH"
cargo build --release --target x86_64-pc-windows-gnu
```

The new binary appears in `target\x86_64-pc-windows-gnu\release\`.

## How it works
- Uses the Lichess **Board API** (token login, game stream, send move).
- **Stockfish** applies the move list; the bot only tracks whose turn it is.
- `--watch` detects each new game's mode and skips non-standard variants.

## Fair play
Use **unrated games** or a **bot account**, and keep `.env` private.
