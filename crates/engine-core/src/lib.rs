//! Platform-independent game simulation for Project Rogue.

/// The current state of the game simulation.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Engine {
    turn: u64,
}

impl Engine {
    /// Creates a new simulation in its initial state.
    #[must_use]
    pub const fn new() -> Self {
        Self { turn: 0 }
    }

    /// Returns the current simulation turn.
    #[must_use]
    pub const fn turn(&self) -> u64 {
        self.turn
    }

    /// Advances the simulation by one turn.
    pub const fn advance_turn(&mut self) {
        self.turn += 1;
    }
}
