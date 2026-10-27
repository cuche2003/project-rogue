# Project Rogue

A terminal-first roguelike built in Rust.

## Architecture

- `crates/game-engine`: deterministic, UI-independent game state and rules.
- `crates/terminal-ui`: Ratatui + Crossterm frontend.

The engine has no terminal or rendering dependencies, so other frontends—including WebAssembly—can be added without changing game rules.

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
cargo build -p game-engine --target wasm32-unknown-unknown
```

## License

MIT
