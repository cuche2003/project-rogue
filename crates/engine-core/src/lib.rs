//! Platform-independent game simulation for Project Rogue.

use bevy_ecs::prelude::{Entity, World};
use std::collections::VecDeque;

mod components;
mod event;
mod map;
mod resources;
mod snapshot;

pub use components::{BlocksMovement, Display, Enemy, GridPosition, Health, Identity, Player};
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
                BlocksMovement,
            ))
            .id();
        let enemy_position = Position {
            x: config.map_width.saturating_sub(2) as i32,
            y: config.map_height.saturating_sub(2) as i32,
        };
        if enemy_position != Position::default() {
            world.spawn((
                Enemy,
                Identity(EntityId(1)),
                GridPosition(enemy_position),
                Health {
                    current: 3,
                    maximum: 3,
                },
                Display {
                    glyph: 'r',
                    name: "Rat".to_owned(),
                },
                BlocksMovement,
            ));
        }

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

        let entities = self
            .world
            .iter_entities()
            .filter_map(|entity| {
                Some(EntitySnapshot {
                    id: entity.get::<Identity>()?.0,
                    position: entity.get::<GridPosition>()?.0,
                    glyph: entity.get::<Display>()?.glyph,
                    name: entity.get::<Display>()?.name.clone(),
                })
            })
            .collect();

        RenderSnapshot {
            phase: self.world.resource::<Phase>().0,
            turn: self.turn(),
            map: MapSnapshot {
                width: map.width(),
                height: map.height(),
                tiles: map.tiles().to_vec(),
            },
            entities,
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

        if let Some(enemy) = self.enemy_at(to) {
            return self.attack_enemy(enemy);
        }

        if self.is_blocked(to, self.player) {
            return CommandResult::Rejected(CommandError::Blocked);
        }

        self.world.entity_mut(self.player).insert(GridPosition(to));
        self.events.push_back(GameEvent::EntityMoved {
            entity: self.player_id(),
            from,
            to,
        });
        self.apply_turn()
    }

    fn enemy_at(&mut self, position: Position) -> Option<Entity> {
        self.world
            .query::<(Entity, &Enemy, &GridPosition)>()
            .iter(&self.world)
            .find_map(|(entity, _, entity_position)| {
                (entity_position.0 == position).then_some(entity)
            })
    }

    fn attack_enemy(&mut self, enemy: Entity) -> CommandResult {
        let target_id = self
            .world
            .get::<Identity>(enemy)
            .expect("enemy entity must have Identity")
            .0;
        let (remaining_hp, died) = {
            let mut health = self
                .world
                .get_mut::<Health>(enemy)
                .expect("enemy entity must have Health");
            health.current = health.current.saturating_sub(1);
            (health.current, health.current == 0)
        };
        self.events.push_back(GameEvent::DamageTaken {
            target: target_id,
            amount: 1,
            remaining_hp,
        });
        if died {
            self.world.despawn(enemy);
            self.events
                .push_back(GameEvent::EntityDied { entity: target_id });
        }
        self.apply_turn()
    }

    fn is_blocked(&mut self, position: Position, moving_entity: Entity) -> bool {
        self.world
            .query::<(Entity, &GridPosition, &BlocksMovement)>()
            .iter(&self.world)
            .any(|(entity, entity_position, _)| {
                entity != moving_entity && entity_position.0 == position
            })
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
    fn enemy_blocks_movement_and_appears_in_snapshot() {
        let mut engine = Engine::new(GameConfig {
            map_width: 4,
            map_height: 3,
            ..GameConfig::default()
        });

        assert_eq!(engine.snapshot().entities.len(), 2);
        assert_eq!(
            engine.dispatch(Command::Move(Direction::Right)),
            CommandResult::Applied
        );
        assert_eq!(
            engine.dispatch(Command::Move(Direction::Down)),
            CommandResult::Applied
        );
        engine.drain_events();
        assert_eq!(
            engine.dispatch(Command::Move(Direction::Right)),
            CommandResult::Applied
        );
        assert_eq!(engine.player_position(), Position { x: 1, y: 1 });
        assert_eq!(engine.turn(), 3);
        assert_eq!(
            engine.drain_events(),
            vec![super::GameEvent::DamageTaken {
                target: super::EntityId(1),
                amount: 1,
                remaining_hp: 2,
            }]
        );
    }

    #[test]
    fn bump_attack_damages_and_despawns_enemy() {
        let mut engine = Engine::new(GameConfig {
            map_width: 4,
            map_height: 3,
            ..GameConfig::default()
        });
        engine.dispatch(Command::Move(Direction::Right));
        engine.dispatch(Command::Move(Direction::Down));
        engine.drain_events();

        assert_eq!(
            engine.dispatch(Command::Move(Direction::Right)),
            CommandResult::Applied
        );
        assert_eq!(engine.snapshot().entities.len(), 2);
        assert_eq!(engine.drain_events().len(), 1);

        engine.dispatch(Command::Move(Direction::Right));
        assert_eq!(engine.snapshot().entities.len(), 2);
        engine.drain_events();

        engine.dispatch(Command::Move(Direction::Right));
        assert_eq!(engine.snapshot().entities.len(), 1);
        assert_eq!(engine.turn(), 5);
        assert_eq!(engine.drain_events().len(), 2);
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
