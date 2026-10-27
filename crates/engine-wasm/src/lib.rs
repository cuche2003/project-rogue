//! WebAssembly bindings for the Project Rogue game engine.

use project_rogue_engine_core::Engine as CoreEngine;
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

    /// Advances the simulation by one turn.
    pub fn advance_turn(&mut self) {
        self.inner
            .dispatch(project_rogue_engine_core::Command::Wait);
    }
}
