# Project Rogue

Terminal-first roguelike built in Rust.

## Architecture

- `crates/engine-core`: deterministic, UI-independent game state and rules backed by `bevy_ecs`.
- `crates/engine-wasm`: WebAssembly bindings for the engine.
- `crates/terminal-ui`: Ratatui + Crossterm frontend.
- `apps/web`: future web frontend.

Engine has no terminal or rendering dependencies, so frontends can evolve independently.

## Run

```sh
cargo run -p terminal-ui
```

Controls: arrows or `hjkl` move and bump-attack; `.` or Space waits; `r` restarts; `q` or `Esc` quits.

Hunt the rat (`r`) as the player (`@`). Each accepted move, attack, or wait lets the rat move or attack once. Three hits defeat it; losing all HP ends the run. The HUD shows HP and turn count, and the combat log records damage and deaths. Resize the terminal to fit the map, or use the player-following viewport on smaller screens (minimum 30 × 12).

## Check

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build -p project-rogue-engine-wasm --target wasm32-unknown-unknown
```

## License

MIT
