# Project Rogue

Terminal-first roguelike built in Rust.

## Architecture

- `crates/engine-core`: deterministic, UI-independent game state and rules.
- `crates/engine-wasm`: WebAssembly bindings for the engine.
- `crates/terminal-ui`: Ratatui + Crossterm frontend.
- `apps/web`: future web frontend.

Engine has no terminal or rendering dependencies, so frontends can evolve independently.

## Run

```sh
cargo run -p terminal-ui
```

Controls: arrows or `hjkl` move, `q` or `Esc` quits.

## Check

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build -p project-rogue-engine-wasm --target wasm32-unknown-unknown
```

## License

MIT
