//! WebAssembly bindings for the Project Rogue game engine.

use project_rogue_engine_core::Engine as CoreEngine;
use project_rogue_engine_core::{Command, CommandResult, Direction, GameEvent, RenderSnapshot};
use wasm_bindgen::prelude::*;

/// JavaScript-facing handle for the game simulation.
#[wasm_bindgen]
pub struct Engine {
    inner: CoreEngine,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl Engine {
    /// Creates a new game simulation.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: CoreEngine::default(),
        }
    }

    /// Returns the current simulation turn.
    #[must_use]
    pub fn turn(&self) -> u64 {
        self.inner.turn()
    }

    /// Returns the current player-visible render state as JSON.
    pub fn snapshot(&self) -> Result<String, JsValue> {
        to_json(&self.inner.snapshot())
    }

    /// Dispatches one named player command and returns result, events, and snapshot as JSON.
    pub fn command(&mut self, action: &str) -> Result<String, JsValue> {
        let command = parse_command(action)?;
        let result = self.inner.dispatch(command);
        let events = self.inner.drain_events();
        to_json(&CommandResponse {
            result,
            events,
            snapshot: self.inner.snapshot(),
        })
    }

    /// Advances the simulation by one turn.
    pub fn advance_turn(&mut self) {
        self.inner.dispatch(Command::Wait);
    }
}

#[derive(serde::Serialize)]
struct CommandResponse {
    result: CommandResult,
    events: Vec<GameEvent>,
    snapshot: RenderSnapshot,
}

fn parse_command(action: &str) -> Result<Command, JsValue> {
    let mut words = action.split_whitespace();
    let command = match words.next() {
        Some("move") => {
            let direction = match words.next() {
                Some("up") => Direction::Up,
                Some("down") => Direction::Down,
                Some("left") => Direction::Left,
                Some("right") => Direction::Right,
                _ => return Err(JsValue::from_str("move requires up, down, left, or right")),
            };
            Command::Move(direction)
        }
        Some("wait") => Command::Wait,
        Some("pickup") => Command::Pickup,
        Some("descend") => Command::Descend,
        Some("restart") => Command::Restart,
        Some("use") => {
            let slot = words
                .next()
                .ok_or_else(|| JsValue::from_str("use requires an inventory slot"))?
                .parse::<u8>()
                .map_err(|_| JsValue::from_str("use slot must be an integer from 0 to 8"))?;
            if slot >= 9 {
                return Err(JsValue::from_str("use slot must be an integer from 0 to 8"));
            }
            Command::UseItem(slot)
        }
        Some(action) => return Err(JsValue::from_str(&format!("unknown command: {action}"))),
        None => return Err(JsValue::from_str("command cannot be empty")),
    };
    if words.next().is_some() {
        return Err(JsValue::from_str("unexpected extra arguments"));
    }
    Ok(command)
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String, JsValue> {
    serde_json::to_string(value).map_err(|error| JsValue::from_str(&error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::parse_command;
    use project_rogue_engine_core::{Command, Direction};

    #[test]
    fn parses_web_commands() {
        assert_eq!(
            parse_command("move right").unwrap(),
            Command::Move(Direction::Right)
        );
        assert_eq!(parse_command("use 2").unwrap(), Command::UseItem(2));
    }
}
