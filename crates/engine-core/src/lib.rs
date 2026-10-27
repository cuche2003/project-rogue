//! Platform-independent game simulation for Project Rogue.

use bevy_ecs::prelude::{Entity, World};
use std::collections::VecDeque;

mod components;
mod event;
mod map;
mod resources;
mod snapshot;

pub use components::{Display, GridPosition, Health, Identity, Player};
pub use event::GameEvent;
pub use map::{GameMap, Position, Tile};
pub use resources::{GameConfig, Map, NextEntityId, Phase, Turn};
pub use snapshot::{EntitySnapshot, HealthSnapshot, MapSnapshot, PlayerSnapshot, RenderSnapshot};

/// Stable game identity independent of ECS storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u64);

/// Direction accepted by game commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Current lifecycle phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamePhase {
    Playing,
    Victory,
    Defeat,
}

/// Player command contracts.
pub mod command;
pub use command::{Command, CommandError, CommandResult};

/// Platform-independent game state backed by Bevy ECS.
pub struct Engine {
    world: World,
    player: Entity,
    seed: u64,
    events: VecDeque<GameEvent>,
}

impl Engine {
    /// Creates a new simulation from configuration.
    #[must_use]
    pub fn new(config: GameConfig) -> Self {
        let mut world = World::new();
        world.insert_resource(Turn::default());
        world.insert_resource(Phase(GamePhase::Playing));
        world.insert_resource(NextEntityId(NextEntityId::default().0 + 1));
        world.insert_resource(Map(GameMap::new(
            config.map_width,
            config.map_height,
            Tile::Floor,
        )));

        let player_id = EntityId(0);
        let player = world
            .spawn((
                Player,
                Identity(player_id),
                GridPosition(Position::default()),
                Health {
                    current: 10,
                    maximum: 10,
                },
                Display {
                    glyph: '@',
                    name: "Player".to_owned(),
                },
            ))
            .id();

        Self {
            world,
            player,
            seed: config.seed,
            events: VecDeque::new(),
        }
    }

    /// Returns a default-configured simulation.
    #[must_use]
    pub fn default_game() -> Self {
        Self::new(GameConfig::default())
    }

    /// Returns the ECS world for controlled engine and mod operations.
    #[must_use]
    pub const fn world(&self) -> &World {
        &self.world
    }

    /// Returns the ECS world for controlled engine and mod operations.
    #[must_use]
    pub const fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Returns the player's stable game identity.
    #[must_use]
    pub fn player_id(&self) -> EntityId {
        self.world
            .get::<Identity>(self.player)
            .expect("player entity must have Identity")
            .0
    }

    /// Returns the player's current position.
    #[must_use]
    pub fn player_position(&self) -> Position {
        self.world
            .get::<GridPosition>(self.player)
            .expect("player entity must have GridPosition")
            .0
    }

    /// Returns current simulation turn.
    #[must_use]
    pub fn turn(&self) -> u64 {
        self.world.resource::<Turn>().0
    }

    /// Returns configured seed.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// Dispatches one frontend command.
    pub fn dispatch(&mut self, command: Command) -> CommandResult {
        if !matches!(self.world.resource::<Phase>().0, GamePhase::Playing)
            && !matches!(command, Command::Restart)
        {
            return CommandResult::Rejected(CommandError::GameOver);
        }

        match command {
            Command::Move(direction) => self.move_player(direction),
            Command::Wait => self.apply_turn(),
            Command::Descend => CommandResult::Rejected(CommandError::NoStairs),
            Command::Restart => {
                *self = Self::default_game();
                CommandResult::Applied
            }
        }
    }

    /// Removes and returns all events emitted since the previous drain.
    pub fn drain_events(&mut self) -> Vec<GameEvent> {
        self.events.drain(..).collect()
    }

    /// Returns a complete UI-neutral view of current state.
    #[must_use]
    pub fn snapshot(&self) -> RenderSnapshot {
        let map = &self.world.resource::<Map>().0;
        let position = self.player_position();
        let health = self
            .world
            .get::<Health>(self.player)
            .expect("player health");

        RenderSnapshot {
            phase: self.world.resource::<Phase>().0,
            turn: self.turn(),
            map: MapSnapshot {
                width: map.width(),
                height: map.height(),
                tiles: map.tiles().to_vec(),
            },
            entities: vec![EntitySnapshot {
                id: self.player_id(),
                position,
                glyph: '@',
                name: "Player".to_owned(),
            }],
            player: PlayerSnapshot {
                id: self.player_id(),
                position,
                health: HealthSnapshot {
                    current: health.current,
                    maximum: health.maximum,
                },
            },
            messages: Vec::new(),
        }
    }

    fn move_player(&mut self, direction: Direction) -> CommandResult {
        let from = self.player_position();
        let to = match direction {
            Direction::Up => Position {
                y: from.y - 1,
                ..from
            },
            Direction::Down => Position {
                y: from.y + 1,
                ..from
            },
            Direction::Left => Position {
                x: from.x - 1,
                ..from
            },
            Direction::Right => Position {
                x: from.x + 1,
                ..from
            },
        };

        if !matches!(self.world.resource::<Map>().0.tile(to), Some(Tile::Floor)) {
            return CommandResult::Rejected(CommandError::Blocked);
        }

        self.world.entity_mut(self.player).insert(GridPosition(to));
        self.apply_turn()
    }

    fn apply_turn(&mut self) -> CommandResult {
        self.world.resource_mut::<Turn>().0 += 1;
        CommandResult::Applied
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::default_game()
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, CommandError, CommandResult, Direction, Engine, GameConfig, Position};

    #[test]
    fn accepted_movement_increments_turn() {
        let mut engine = Engine::new(GameConfig {
            map_width: 4,
            map_height: 4,
            ..GameConfig::default()
        });

        assert_eq!(
            engine.dispatch(Command::Move(Direction::Right)),
            CommandResult::Applied
        );
        assert_eq!(engine.player_position(), Position { x: 1, y: 0 });
        assert_eq!(engine.turn(), 1);
    }

    #[test]
    fn blocked_movement_does_not_increment_turn() {
        let mut engine = Engine::default();

        assert_eq!(
            engine.dispatch(Command::Move(Direction::Up)),
            CommandResult::Rejected(CommandError::Blocked)
        );
        assert_eq!(engine.turn(), 0);
    }

    #[test]
    fn same_seed_and_commands_produce_same_snapshot() {
        let mut first = Engine::new(GameConfig::default());
        let mut second = Engine::new(GameConfig::default());
        let commands = [
            Command::Move(Direction::Right),
            Command::Move(Direction::Down),
            Command::Wait,
        ];

        for command in commands {
            assert_eq!(first.dispatch(command), second.dispatch(command));
        }
        assert_eq!(first.snapshot(), second.snapshot());
    }
}
