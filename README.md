# Project Rogue

Terminal-first roguelike built in Rust.

## Architecture

- `crates/engine-core`: deterministic, UI-independent game state and rules backed by `bevy_ecs`.
- `crates/engine-wasm`: WebAssembly bindings for the engine.
- `crates/terminal-ui`: Ratatui + Crossterm frontend. Replay mode steps with Space/Enter or plays automatically with `p`.
- `apps/web`: future web frontend.

Engine has no terminal or rendering dependencies, so frontends can evolve independently.

## Run

```sh
cargo run -p terminal-ui
cargo run -p terminal-ui -- --record run.ron
cargo run -p terminal-ui -- --replay run.ron
```

Controls: arrows or `hjkl` move and bump-attack; `.` or Space waits; `g` picks up items; `1`-`9` uses inventory items; `>` descends; `r` restarts; `q` or `Esc` quits.

Explore seeded rooms and corridors as player (`@`), survive rats (`r`), and find stairs (`>`). Stand on stairs and press `>` to descend and win. Losing all HP ends the run in defeat. Each accepted move, attack, or wait gives enemies a turn; blocked moves do not spend a turn. HUD shows HP, turn count, and objective; log records messages, damage, deaths, and victory or defeat.

Successful descent ends the run immediately without enemy retaliation. Core runs ordered Bevy ECS stages: player action, cleanup of enemies killed by the player, enemy actions, cleanup, outcome, visibility. Enemy pathfinding uses a shared deterministic distance field and stable entity ordering.

Default run uses seed `0`. Restart (`r`) resets the same seed and dungeon, not a fresh random map. Actor content lives in `crates/engine-core/content/core.ron`: stable namespaced IDs, validated stats, spawn counts, and Rust-defined behavior kinds. Hosts can parse RON with `ContentPack::from_ron`, compose packs with `ContentPack::compose`, and start with `Engine::with_content`; spawn order is deterministic for a given seed and pack. Terminal loads sorted `mods/*.ron` files from project-root `mods/`, composes them over core content, and fails before starting when parsing, IDs, spawns, items, or map capacity are invalid. Item content defines namespaced items, stack limits, effects, power, and deterministic item spawns. Player inventory is engine-owned; `g` picks up the item under the player and `1`-`9` uses items. Filesystem loading stays outside core; no Lua runtime enters simulation yet. Core computes field of view: unexplored tiles stay blank, remembered terrain stays dim, and currently visible terrain is brighter. Only visible entities are supplied to the frontend (player always included); an unseen rat is not necessarily dead.

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
