//! Platform-independent deterministic simulation for Project Rogue.

use bevy_ecs::prelude::{Entity, Schedule, World};

mod command;
mod components;
mod content;
mod event;
mod generation;
mod map;
mod resources;
mod snapshot;
mod systems;
mod visibility;

pub use command::{Command, CommandError, CommandResult};
pub use components::{
    AttackDamage, Behavior, BlocksMovement, Display, Enemy, GridPosition, Health, Identity, Player,
};
pub use content::{ActorDefinition, BehaviorKind, ContentError, ContentPack};
pub use event::GameEvent;
pub use map::{GameMap, Position, Tile};
use resources::{Events, PlayerEntity, TurnCommand};
pub use resources::{GameConfig, Map, NextEntityId, Phase, Turn};
pub use snapshot::{EntitySnapshot, HealthSnapshot, MapSnapshot, PlayerSnapshot, RenderSnapshot};
pub use systems::TurnSet;
use visibility::Visibility;

/// Stable identity independent of ECS storage; unique within one run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u64);

/// Cardinal movement directions. Y increases south.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Current run state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamePhase {
    Playing,
    Victory,
    Defeat,
}

/// Invalid dimensions supplied at the engine boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigError;

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("map width and height must each be in 8..=256")
    }
}
impl std::error::Error for ConfigError {}

/// ECS simulation facade. Frontends dispatch commands and consume snapshots/events.
pub struct Engine {
    world: World,
    schedule: Schedule,
    player: Entity,
    config: GameConfig,
    content: ContentPack,
}

impl Engine {
    /// Creates a run. Panics for invalid dimensions; use `try_new` for untrusted configuration.
    #[must_use]
    pub fn new(config: GameConfig) -> Self {
        Self::with_content(config, ContentPack::core()).expect("built-in content is valid")
    }

    /// Validates dimensions and built-in content before allocating or generating a dungeon.
    pub fn try_new(config: GameConfig) -> Result<Self, ConfigError> {
        Self::with_content(config, ContentPack::core())
    }

    /// Creates a run with validated host or mod content.
    pub fn with_content(config: GameConfig, content: ContentPack) -> Result<Self, ConfigError> {
        if !(8..=256).contains(&config.map_width) || !(8..=256).contains(&config.map_height) {
            return Err(ConfigError);
        }
        let (map, start, enemy_position) = generation::generate(config);
        let player_content = content.actor("core:player").ok_or(ConfigError)?;
        let rat = content.actor("core:rat").ok_or(ConfigError)?;
        let sentinel_position = content.actor("core:sentinel").and_then(|_| {
            map.tiles().iter().enumerate().find_map(|(index, tile)| {
                if *tile != Tile::Floor {
                    return None;
                }
                let position = Position {
                    x: (index % usize::from(map.width())) as i32,
                    y: (index / usize::from(map.width())) as i32,
                };
                (position != start && position != enemy_position).then_some(position)
            })
        });
        let mut world = World::new();
        world.insert_resource(Turn::default());
        world.insert_resource(Phase(GamePhase::Playing));
        world.insert_resource(Map(map));
        world.insert_resource(Events::default());
        world.insert_resource(Visibility::default());
        let mut ids = NextEntityId::default();
        let player = world
            .spawn((
                Player,
                Identity(ids.allocate()),
                GridPosition(start),
                Health {
                    current: player_content.max_health,
                    maximum: player_content.max_health,
                },
                AttackDamage(player_content.attack_damage),
                Display {
                    glyph: player_content.glyph,
                    name: player_content.name.clone(),
                },
                BlocksMovement,
            ))
            .id();
        world.spawn((
            Enemy,
            Behavior(rat.behavior),
            AttackDamage(rat.attack_damage),
            Identity(ids.allocate()),
            GridPosition(enemy_position),
            Health {
                current: rat.max_health,
                maximum: rat.max_health,
            },
            Display {
                glyph: rat.glyph,
                name: rat.name.clone(),
            },
            BlocksMovement,
        ));
        if let (Some(position), Some(sentinel)) =
            (sentinel_position, content.actor("core:sentinel"))
        {
            world.spawn((
                Enemy,
                Behavior(sentinel.behavior),
                AttackDamage(sentinel.attack_damage),
                Identity(ids.allocate()),
                GridPosition(position),
                Health {
                    current: sentinel.max_health,
                    maximum: sentinel.max_health,
                },
                Display {
                    glyph: sentinel.glyph,
                    name: sentinel.name.clone(),
                },
                BlocksMovement,
            ));
        }
        world.insert_resource(ids);
        world.insert_resource(PlayerEntity(player));
        systems::update_visibility(&mut world);
        Ok(Self {
            world,
            schedule: systems::schedule(),
            player,
            config,
            content,
        })
    }

    #[must_use]
    pub fn default_game() -> Self {
        Self::default()
    }

    /// Read-only ECS access for engine tools. Frontends should use `snapshot`.
    #[must_use]
    pub const fn world(&self) -> &World {
        &self.world
    }

    /// Trusted engine tooling only: callers must preserve entity/map invariants.
    /// Visibility is refreshed on the next accepted turn, not by direct world edits.
    #[must_use]
    pub const fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    #[must_use]
    pub fn player_id(&self) -> EntityId {
        self.world
            .get::<Identity>(self.player)
            .expect("player identity")
            .0
    }

    #[must_use]
    pub fn player_position(&self) -> Position {
        self.world
            .get::<GridPosition>(self.player)
            .expect("player position")
            .0
    }

    #[must_use]
    pub fn turn(&self) -> u64 {
        self.world.resource::<Turn>().0
    }

    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.config.seed
    }

    /// Rejected commands spend no turn. Restart reproduces the original seed/configuration.
    pub fn dispatch(&mut self, command: Command) -> CommandResult {
        if command == Command::Restart {
            *self = Self::with_content(self.config, self.content.clone())
                .expect("validated configuration");
            return CommandResult::Applied;
        }
        if let Err(error) = systems::validate(&mut self.world, command) {
            return CommandResult::Rejected(error);
        }
        let pending = self.drain_events();
        let mut observed = self.observed_entities();
        self.world.insert_resource(TurnCommand(command));
        self.schedule.run(&mut self.world);
        observed.extend(self.observed_entities());
        // Events share the snapshot's information boundary; hidden AI paths stay private.
        let map = &self.world.resource::<Map>().0;
        let sight = self.world.resource::<Visibility>();
        let filtered: Vec<_> = self
            .world
            .resource::<Events>()
            .0
            .iter()
            .filter(|event| match event {
                GameEvent::EntityMoved { entity, from, to } => {
                    *entity == self.player_id() || (sight.sees(map, *from) && sight.sees(map, *to))
                }
                GameEvent::DamageTaken { target, .. } => observed.contains(target),
                GameEvent::EntityDied { entity } => observed.contains(entity),
                _ => true,
            })
            .cloned()
            .collect();
        self.world.resource_mut::<Events>().0 = pending;
        self.world.resource_mut::<Events>().0.extend(filtered);
        CommandResult::Applied
    }

    fn observed_entities(&self) -> std::collections::HashSet<EntityId> {
        let sight = self.world.resource::<Visibility>();
        let map = &self.world.resource::<Map>().0;
        self.world
            .iter_entities()
            .filter_map(|entity| {
                sight
                    .sees(map, entity.get::<GridPosition>()?.0)
                    .then_some(entity.get::<Identity>()?.0)
            })
            .collect()
    }

    /// Returns ordered player-observable events since the last drain.
    /// Hidden actors' movement is omitted instead of exposing unseen coordinates.
    pub fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.world.resource_mut::<Events>().0)
    }

    /// Player knowledge only: unexplored terrain and hidden entities are never exposed.
    #[must_use]
    pub fn snapshot(&self) -> RenderSnapshot {
        let map = &self.world.resource::<Map>().0;
        let sight = self.world.resource::<Visibility>();
        let mut entities: Vec<_> = self
            .world
            .iter_entities()
            .filter_map(|entity| {
                let position = entity.get::<GridPosition>()?.0;
                if entity.id() != self.player && !sight.sees(map, position) {
                    return None;
                }
                let display = entity.get::<Display>()?;
                Some(EntitySnapshot {
                    id: entity.get::<Identity>()?.0,
                    position,
                    glyph: display.glyph,
                    name: display.name.clone(),
                })
            })
            .collect();
        entities.sort_unstable_by_key(|entity| entity.id.0);
        let health = self
            .world
            .get::<Health>(self.player)
            .expect("player health");
        RenderSnapshot {
            phase: self.world.resource::<Phase>().0,
            turn: self.turn(),
            seed: self.seed(),
            map: MapSnapshot {
                width: map.width(),
                height: map.height(),
                tiles: sight.remembered.clone(),
                visible: sight.visible.clone(),
            },
            entities,
            player: PlayerSnapshot {
                id: self.player_id(),
                position: self.player_position(),
                health: HealthSnapshot {
                    current: health.current,
                    maximum: health.maximum,
                },
            },
            messages: Vec::new(),
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new(GameConfig::default())
    }
}

#[cfg(test)]
mod tests;
