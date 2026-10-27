#![forbid(unsafe_code)]

/// Grid coordinate used by game state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// UI-independent game state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    player: Position,
    turn: u64,
}

impl Game {
    pub fn new() -> Self {
        Self {
            player: Position { x: 0, y: 0 },
            turn: 0,
        }
    }

    pub fn player_position(&self) -> Position {
        self.player
    }

    pub fn turn(&self) -> u64 {
        self.turn
    }

    pub fn step(&mut self, direction: Direction) {
        match direction {
            Direction::Up => self.player.y -= 1,
            Direction::Down => self.player.y += 1,
            Direction::Left => self.player.x -= 1,
            Direction::Right => self.player.x += 1,
        }
        self.turn += 1;
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Direction, Game, Position};

    #[test]
    fn step_updates_position_and_turn() {
        let mut game = Game::new();

        game.step(Direction::Right);
        game.step(Direction::Down);

        assert_eq!(game.player_position(), Position { x: 1, y: 1 });
        assert_eq!(game.turn(), 2);
    }
}
