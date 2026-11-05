# Repo posture for agents (see the setup-rust-skills skill).

- Edition: 2021
- MSRV: 1.74 (needed for the Cargo `[lints]` table)
- Async runtime: none (this bot uses blocking I/O only)
- no_std: no
- Unsafe policy: forbidden (`unsafe_code = "forbid"`)

## Commands
- Build: `cargo build --release`
- Test:  `cargo test`
- Lint:  `cargo clippy --all-targets`
- Format:`cargo fmt`

## Notes
- Chess rules are NOT reimplemented here. Stockfish applies the move list via
  `position startpos moves ...`; we only track "whose turn" with ply parity.
- Secrets live in `.env` (gitignored). Identity comes from `GET /api/account`.
