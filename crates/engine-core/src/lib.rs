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
        self.damage(enemy);
        self.apply_turn()
    }

    fn damage(&mut self, target: Entity) {
        let target_id = self
            .world
            .get::<Identity>(target)
            .expect("combat target must have Identity")
            .0;
        let remaining_hp = {
            let mut health = self
                .world
                .get_mut::<Health>(target)
                .expect("combat target must have Health");
            health.current = health.current.saturating_sub(1);
            health.current
        };
        self.events.push_back(GameEvent::DamageTaken {
            target: target_id,
            amount: 1,
            remaining_hp,
        });
        if remaining_hp == 0 {
            self.events
                .push_back(GameEvent::EntityDied { entity: target_id });
            if target == self.player {
                // Retain player data for the final snapshot.
                self.world.resource_mut::<Phase>().0 = GamePhase::Defeat;
                self.events.push_back(GameEvent::Defeat);
            } else {
                self.world.despawn(target);
            }
        }
    }

    fn is_blocked(&mut self, position: Position, moving_entity: Entity) -> bool {
        self.world
            .query::<(Entity, &GridPosition, &BlocksMovement)>()
            .iter(&self.world)
            .any(|(entity, entity_position, _)| {
                entity != moving_entity && entity_position.0 == position
            })
    }

    fn enemy_turns(&mut self) {
        let mut enemies: Vec<_> = self
            .world
            .query::<(Entity, &Identity, &Enemy)>()
            .iter(&self.world)
            .map(|(entity, identity, _)| (identity.0, entity))
            .collect();
        enemies.sort_unstable_by_key(|(id, _)| id.0);

        for (_, enemy) in enemies {
            if self.world.resource::<Phase>().0 != GamePhase::Playing {
                break;
            }
            let from = self
                .world
                .get::<GridPosition>(enemy)
                .expect("enemy must have GridPosition")
                .0;
            let player = self.player_position();
            if from.x.abs_diff(player.x) + from.y.abs_diff(player.y) == 1 {
                self.damage(self.player);
                continue;
            }

            // ponytail: greedy chase can stall at walls; use pathfinding when dungeon layouts need it.
            let candidates = [
                Position {
                    x: from.x + (player.x - from.x).signum(),
                    ..from
                },
                Position {
                    y: from.y + (player.y - from.y).signum(),
                    ..from
                },
            ];
            for to in candidates {
                if to == from
                    || !matches!(self.world.resource::<Map>().0.tile(to), Some(Tile::Floor))
                    || self.is_blocked(to, enemy)
                {
                    continue;
                }
                self.world.entity_mut(enemy).insert(GridPosition(to));
                let entity = self.world.get::<Identity>(enemy).expect("enemy identity").0;
                self.events
                    .push_back(GameEvent::EntityMoved { entity, from, to });
                break;
            }
        }
    }

    fn apply_turn(&mut self) -> CommandResult {
        self.enemy_turns();
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
    use super::*;

    fn adjacent_enemy_game() -> Engine {
        let mut engine = Engine::default();
        let enemy = engine
            .world
            .query::<(Entity, &Enemy)>()
            .iter(&engine.world)
            .next()
            .unwrap()
            .0;
        engine
            .world
            .entity_mut(enemy)
            .insert(GridPosition(Position { x: 1, y: 0 }));
        engine
    }

    #[test]
    fn accepted_movement_increments_turn_and_moves_enemy_once() {
        let mut engine = Engine::default();
        assert_eq!(
            engine.dispatch(Command::Move(Direction::Right)),
            CommandResult::Applied
        );
        assert_eq!(engine.turn(), 1);
        assert_eq!(engine.player_position(), Position { x: 1, y: 0 });
        let snapshot = engine.snapshot();
        let enemy = snapshot
            .entities
            .iter()
            .find(|entity| entity.id == EntityId(1))
            .unwrap();
        assert_eq!(enemy.position, Position { x: 37, y: 18 });
        assert_eq!(engine.drain_events().len(), 2);
    }

    #[test]
    fn rejected_commands_and_restart_do_not_run_enemy_turns() {
        let mut engine = adjacent_enemy_game();
        let initial = engine.snapshot();
        for command in [Command::Move(Direction::Up), Command::Descend] {
            assert!(matches!(
                engine.dispatch(command),
                CommandResult::Rejected(_)
            ));
            assert_eq!(engine.snapshot(), initial);
            assert!(engine.drain_events().is_empty());
        }
        assert_eq!(engine.dispatch(Command::Restart), CommandResult::Applied);
        assert_eq!(engine.turn(), 0);
        assert_eq!(engine.snapshot().player.health.current, 10);
        assert!(engine.drain_events().is_empty());
    }

    #[test]
    fn bump_attack_resolves_before_enemy_response_and_death_frees_tile() {
        let mut engine = adjacent_enemy_game();
        for hp in [2, 1] {
            assert_eq!(
                engine.dispatch(Command::Move(Direction::Right)),
                CommandResult::Applied
            );
            assert_eq!(engine.player_position(), Position { x: 0, y: 0 });
            assert_eq!(
                engine.drain_events(),
                vec![
                    GameEvent::DamageTaken {
                        target: EntityId(1),
                        amount: 1,
                        remaining_hp: hp
                    },
                    GameEvent::DamageTaken {
                        target: EntityId(0),
                        amount: 1,
                        remaining_hp: 7 + hp
                    },
                ]
            );
        }
        engine.dispatch(Command::Move(Direction::Right));
        assert_eq!(
            engine.drain_events(),
            vec![
                GameEvent::DamageTaken {
                    target: EntityId(1),
                    amount: 1,
                    remaining_hp: 0
                },
                GameEvent::EntityDied {
                    entity: EntityId(1)
                },
            ]
        );
        assert_eq!(engine.snapshot().entities.len(), 1);
        assert_eq!(engine.snapshot().player.health.current, 8);
        assert_eq!(engine.turn(), 3);
        engine.dispatch(Command::Move(Direction::Right));
        assert_eq!(engine.player_position(), Position { x: 1, y: 0 });
    }

    #[test]
    fn waiting_allows_enemy_attack_and_defeat_rejects_future_turns() {
        let mut engine = adjacent_enemy_game();
        for _ in 0..9 {
            assert_eq!(engine.dispatch(Command::Wait), CommandResult::Applied);
        }
        engine.drain_events();
        assert_eq!(engine.dispatch(Command::Wait), CommandResult::Applied);
        assert_eq!(
            engine.drain_events(),
            vec![
                GameEvent::DamageTaken {
                    target: EntityId(0),
                    amount: 1,
                    remaining_hp: 0
                },
                GameEvent::EntityDied {
                    entity: EntityId(0)
                },
                GameEvent::Defeat,
            ]
        );
        let final_state = engine.snapshot();
        assert_eq!(final_state.phase, GamePhase::Defeat);
        assert_eq!(final_state.player.health.current, 0);
        assert_eq!(final_state.turn, 10);
        assert_eq!(
            engine.dispatch(Command::Wait),
            CommandResult::Rejected(CommandError::GameOver)
        );
        assert_eq!(engine.snapshot(), final_state);
        assert!(engine.drain_events().is_empty());
    }

    #[test]
    fn enemy_uses_vertical_fallback_and_respects_walls_and_occupants() {
        let mut engine = Engine::default();
        let enemy = engine
            .world
            .query::<(Entity, &Enemy)>()
            .iter(&engine.world)
            .next()
            .unwrap()
            .0;
        engine
            .world
            .entity_mut(enemy)
            .insert(GridPosition(Position { x: 2, y: 2 }));
        engine
            .world
            .resource_mut::<Map>()
            .0
            .set_tile(Position { x: 1, y: 2 }, Tile::Wall);
        engine.dispatch(Command::Wait);
        assert_eq!(
            engine.world.get::<GridPosition>(enemy).unwrap().0,
            Position { x: 2, y: 1 }
        );
        engine
            .world
            .resource_mut::<Map>()
            .0
            .set_tile(Position { x: 1, y: 1 }, Tile::Wall);
        engine
            .world
            .spawn((BlocksMovement, GridPosition(Position { x: 2, y: 0 })));
        engine.drain_events();
        engine.dispatch(Command::Wait);
        assert_eq!(
            engine.world.get::<GridPosition>(enemy).unwrap().0,
            Position { x: 2, y: 1 }
        );
        assert!(engine.drain_events().is_empty());
    }

    #[test]
    fn enemy_order_uses_stable_identity_not_spawn_order() {
        let mut engine = Engine::default();
        for (id, position) in [(3, Position { x: 2, y: 1 }), (2, Position { x: 1, y: 2 })] {
            engine.world.spawn((
                Enemy,
                Identity(EntityId(id)),
                GridPosition(position),
                BlocksMovement,
            ));
        }
        engine.dispatch(Command::Wait);
        let ids: Vec<_> = engine
            .drain_events()
            .into_iter()
            .map(|event| {
                let GameEvent::EntityMoved { entity, .. } = event else {
                    panic!("expected movement")
                };
                entity
            })
            .collect();
        assert_eq!(ids, vec![EntityId(1), EntityId(2), EntityId(3)]);
    }

    #[test]
    fn same_seed_and_commands_produce_same_snapshot_and_events() {
        let mut first = Engine::default();
        let mut second = Engine::default();
        for command in [
            Command::Move(Direction::Right),
            Command::Move(Direction::Down),
            Command::Wait,
        ] {
            assert_eq!(first.dispatch(command), second.dispatch(command));
            assert_eq!(first.snapshot(), second.snapshot());
            assert_eq!(first.drain_events(), second.drain_events());
        }
    }
}
