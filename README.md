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

Controls: arrows or `hjkl` move and bump-attack; `.` or Space waits; `>` descends; `r` restarts; `q` or `Esc` quits.

Explore seeded rooms and corridors as player (`@`), survive rats (`r`), and find stairs (`>`). Stand on stairs and press `>` to descend and win. Losing all HP ends the run in defeat. Each accepted move, attack, or wait gives enemies a turn; blocked moves do not spend a turn. HUD shows HP, turn count, and objective; log records messages, damage, deaths, and victory or defeat.

Successful descent ends the run immediately without enemy retaliation. Core runs ordered Bevy ECS stages: player action, cleanup of enemies killed by the player, enemy actions, cleanup, outcome, visibility. Enemy pathfinding uses a shared deterministic distance field and stable entity ordering.

Default run uses seed `0`. Restart (`r`) resets the same seed and dungeon, not a fresh random map. Actor content lives in `crates/engine-core/content/core.ron`: stable namespaced IDs, validated stats, and Rust-defined behavior kinds. Hosts can load additional RON text with `ContentPack::from_ron` and start with `Engine::with_content`; no filesystem or Lua runtime enters simulation yet. Core computes field of view: unexplored tiles stay blank, remembered terrain stays dim, and currently visible terrain is brighter. Only visible entities are supplied to the frontend (player always included); an unseen rat is not necessarily dead.

Resize terminal to fit the map, or use the player-following viewport on smaller screens. Minimum 30 x 12 keeps status, recent log, and all controls visible.

## Check

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build -p project-rogue-engine-wasm --target wasm32-unknown-unknown
```

## License

MIT
