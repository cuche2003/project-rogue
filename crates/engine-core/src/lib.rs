//! Platform-independent game simulation for Project Rogue.

/// Grid coordinate used by game state.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

/// Direction accepted by game commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// The current state of the game simulation.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Engine {
    player: Position,
    turn: u64,
}

impl Engine {
    /// Creates a new simulation in its initial state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            player: Position { x: 0, y: 0 },
            turn: 0,
        }
    }

    /// Returns the player's current position.
    #[must_use]
    pub const fn player_position(&self) -> Position {
        self.player
    }

    /// Returns the current simulation turn.
    #[must_use]
    pub const fn turn(&self) -> u64 {
        self.turn
    }

    /// Advances the simulation by one turn in given direction.
    pub const fn step(&mut self, direction: Direction) {
        match direction {
            Direction::Up => self.player.y -= 1,
            Direction::Down => self.player.y += 1,
            Direction::Left => self.player.x -= 1,
            Direction::Right => self.player.x += 1,
        }
        self.turn += 1;
    }

    /// Advances the simulation by one turn.
    pub const fn advance_turn(&mut self) {
        self.turn += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::{Direction, Engine, Position};

    #[test]
    fn step_updates_position_and_turn() {
        let mut engine = Engine::new();

        engine.step(Direction::Right);
        engine.step(Direction::Down);

        assert_eq!(engine.player_position(), Position { x: 1, y: 1 });
        assert_eq!(engine.turn(), 2);
    }
}
