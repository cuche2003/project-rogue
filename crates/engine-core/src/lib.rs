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
    AttackDamage, Behavior, BlocksMovement, Display, Enemy, GridPosition, Health, Identity, Item,
    Player,
};
pub use content::{
    ActorDefinition, BehaviorKind, ContentError, ContentPack, ItemDefinition, ItemEffect,
    ItemSpawnDefinition,
};
pub use event::GameEvent;
pub use map::{GameMap, Position, Tile};
use resources::{Content, Events, Floor, GroundItems, Inventory, PlayerEntity, TurnCommand};
pub use resources::{GameConfig, Map, NextEntityId, Phase, Turn};
pub use snapshot::{
    EntitySnapshot, HealthSnapshot, ItemSnapshot, MapSnapshot, PlayerSnapshot, RenderSnapshot,
};
pub use systems::TurnSet;

const REPLAY_VERSION: u16 = 1;

/// Failure while serializing or parsing a replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    Serialize(String),
    Parse(String),
    UnsupportedVersion(u16),
    Playback(String),
    Diverged { command: usize },
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialize(error) => write!(f, "replay serialization failed: {error}"),
            Self::Parse(error) => write!(f, "replay parse failed: {error}"),
            Self::UnsupportedVersion(version) => write!(f, "unsupported replay version {version}"),
            Self::Playback(error) => write!(f, "replay playback failed: {error}"),
            Self::Diverged { command } => write!(f, "replay diverged after command {command}"),
        }
    }
}

impl std::error::Error for ReplayError {}

/// Deterministic command recording for reproducing a run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Replay {
    version: u16,
    config: GameConfig,
    content: ContentPack,
    commands: Vec<Command>,
}

impl Replay {
    /// Serializes replay as versioned RON.
    pub fn to_ron(&self) -> Result<String, ReplayError> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|error| ReplayError::Serialize(error.to_string()))
    }

    /// Parses a versioned RON replay.
    pub fn from_ron(source: &str) -> Result<Self, ReplayError> {
        let replay: Self =
            ron::from_str(source).map_err(|error| ReplayError::Parse(error.to_string()))?;
        if replay.version != REPLAY_VERSION {
            return Err(ReplayError::UnsupportedVersion(replay.version));
        }
        Ok(replay)
    }

    /// Returns recorded commands in dispatch order, including rejected commands and restarts.
    #[must_use]
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// Creates fresh engine at replay start state.
    pub fn start(&self) -> Result<Engine, EngineError> {
        Engine::with_content(self.config, self.content.clone())
    }

    /// Rebuilds a run by dispatching every recorded command from its original start state.
    pub fn playback(&self) -> Result<Engine, EngineError> {
        let mut engine = self.start()?;
        for &command in &self.commands {
            engine.dispatch(command);
        }
        Ok(engine)
    }

    /// Runs replay twice and checks every command produces identical state and events.
    pub fn verify(&self) -> Result<(), ReplayError> {
        let mut first = self
            .start()
            .map_err(|error| ReplayError::Playback(error.to_string()))?;
        let mut second = self
            .start()
            .map_err(|error| ReplayError::Playback(error.to_string()))?;
        for (index, &command) in self.commands.iter().enumerate() {
            if first.dispatch(command) != second.dispatch(command)
                || first.snapshot() != second.snapshot()
                || first.drain_events() != second.drain_events()
            {
                return Err(ReplayError::Diverged { command: index });
            }
        }
        Ok(())
    }
}
use visibility::Visibility;

/// Stable identity independent of ECS storage; unique within one run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub struct EntityId(pub u64);

/// Cardinal movement directions. Y increases south.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Current run state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
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

/// Failure while preparing a run from configuration and content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    Config(ConfigError),
    Content(ContentError),
    InsufficientSpawnTiles { requested: usize, available: usize },
    InsufficientItemTiles { requested: usize, available: usize },
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(error) => error.fmt(f),
            Self::Content(error) => error.fmt(f),
            Self::InsufficientSpawnTiles {
                requested,
                available,
            } => write!(
                f,
                "content requests {requested} actors but map has only {available} spawn tiles"
            ),
            Self::InsufficientItemTiles {
                requested,
                available,
            } => write!(
                f,
                "content requests {requested} items but only {available} spawn tiles remain"
            ),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<ConfigError> for EngineError {
    fn from(error: ConfigError) -> Self {
        Self::Config(error)
    }
}

impl From<ContentError> for EngineError {
    fn from(error: ContentError) -> Self {
        Self::Content(error)
    }
}

/// ECS simulation facade. Frontends dispatch commands and consume snapshots/events.
pub struct Engine {
    world: World,
    schedule: Schedule,
    player: Entity,
    config: GameConfig,
    content: ContentPack,
    replay: Option<Vec<Command>>,
}

impl Engine {
    /// Creates a run. Panics for invalid configuration; use `try_new` for untrusted configuration.
    #[must_use]
    pub fn new(config: GameConfig) -> Self {
        Self::with_content(config, ContentPack::core()).expect("built-in content is valid")
    }

    /// Validates configuration and built-in content before allocating or generating a dungeon.
    pub fn try_new(config: GameConfig) -> Result<Self, ConfigError> {
        Self::with_content(config, ContentPack::core()).map_err(|error| match error {
            EngineError::Config(error) => error,
            EngineError::Content(_)
            | EngineError::InsufficientSpawnTiles { .. }
            | EngineError::InsufficientItemTiles { .. } => {
                unreachable!("built-in content is valid for default dimensions")
            }
        })
    }

    /// Creates a run with validated host or mod content.
    pub fn with_content(config: GameConfig, content: ContentPack) -> Result<Self, EngineError> {
        if !(8..=256).contains(&config.map_width) || !(8..=256).contains(&config.map_height) {
            return Err(ConfigError.into());
        }
        if config.floor_count == 0 {
            return Err(ConfigError.into());
        }
        let (map, start, _enemy_position) = generation::generate(config);
        let player_content = content
            .actor("core:player")
            .ok_or_else(|| ContentError::InvalidId("core:player".to_owned()))?;
        let mut spawn_positions = map
            .tiles()
            .iter()
            .enumerate()
            .filter_map(|(index, tile)| {
                (*tile == Tile::Floor).then_some(Position {
                    x: (index % usize::from(map.width())) as i32,
                    y: (index / usize::from(map.width())) as i32,
                })
            })
            .filter(|position| *position != start)
            .collect::<Vec<_>>();
        spawn_positions.sort_unstable_by_key(|position| {
            std::cmp::Reverse(position.x.abs_diff(start.x) + position.y.abs_diff(start.y))
        });
        let requested_spawns: usize = content
            .spawns()
            .iter()
            .map(|spawn| spawn.count as usize)
            .sum();
        if requested_spawns > spawn_positions.len() {
            return Err(EngineError::InsufficientSpawnTiles {
                requested: requested_spawns,
                available: spawn_positions.len(),
            });
        }
        // ponytail: floor-only item placement; add explicit item-on-stairs rules when content needs them.
        let available_item_tiles = spawn_positions.len() - requested_spawns;
        let requested_items: usize = content
            .item_spawns()
            .iter()
            .map(|spawn| spawn.count as usize)
            .sum();
        if requested_items > available_item_tiles {
            return Err(EngineError::InsufficientItemTiles {
                requested: requested_items,
                available: available_item_tiles,
            });
        }
        let item_spawn_positions = spawn_positions
            .iter()
            .skip(requested_spawns)
            .copied()
            .collect::<Vec<_>>();
        let mut world = World::new();
        world.insert_resource(Turn::default());
        world.insert_resource(Floor(1));
        world.insert_resource(Phase(GamePhase::Playing));
        world.insert_resource(Map(map));
        world.insert_resource(Events::default());
        world.insert_resource(GroundItems::default());
        world.insert_resource(Inventory::default());
        world.insert_resource(Content(content.clone()));
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
        let mut spawn_index = 0;
        for spawn in content.spawns() {
            let actor = content
                .actor(&spawn.actor_id)
                .expect("validated spawn actor");
            for _ in 0..spawn.count {
                let position = spawn_positions[spawn_index];
                spawn_index += 1;
                world.spawn((
                    Enemy,
                    Behavior(actor.behavior),
                    AttackDamage(actor.attack_damage),
                    Identity(ids.allocate()),
                    GridPosition(position),
                    Health {
                        current: actor.max_health,
                        maximum: actor.max_health,
                    },
                    Display {
                        glyph: actor.glyph,
                        name: actor.name.clone(),
                    },
                    BlocksMovement,
                ));
            }
        }
        let mut item_index = 0;
        let item_spawns = content
            .item_spawns()
            .iter()
            .flat_map(|spawn| (0..spawn.count).map(|_| spawn.item_id.clone()))
            .map(|item_id| {
                let position = item_spawn_positions[item_index];
                item_index += 1;
                resources::GroundItem {
                    id: ids.allocate(),
                    item_id,
                    position,
                    quantity: 1,
                }
            })
            .collect();
        world.insert_resource(GroundItems(item_spawns));
        world.insert_resource(ids);
        world.insert_resource(PlayerEntity(player));
        systems::update_visibility(&mut world);
        Ok(Self {
            world,
            schedule: systems::schedule(),
            player,
            config,
            content,
            replay: None,
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

    /// Starts recording commands from current engine state.
    pub fn start_recording(&mut self) {
        self.replay = Some(Vec::new());
    }

    /// Stops recording and returns original run configuration plus commands.
    pub fn stop_recording(&mut self) -> Option<Replay> {
        self.replay.take().map(|commands| Replay {
            version: REPLAY_VERSION,
            config: self.config,
            content: self.content.clone(),
            commands,
        })
    }

    /// Rejected commands spend no turn. Restart reproduces the original seed/configuration.
    pub fn dispatch(&mut self, command: Command) -> CommandResult {
        if let Some(commands) = &mut self.replay {
            commands.push(command);
        }
        if command == Command::Restart {
            let replay = self.replay.take();
            *self = Self::with_content(self.config, self.content.clone())
                .expect("validated configuration");
            self.replay = replay;
            return CommandResult::Applied;
        }
        if let Err(error) = systems::validate(&mut self.world, command) {
            return CommandResult::Rejected(error);
        }
        if command == Command::Descend {
            self.descend();
            return CommandResult::Applied;
        }
        let pending = self.drain_events();
        self.world.insert_resource(TurnCommand(command));
        self.schedule.run(&mut self.world);
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
                GameEvent::DamageTaken {
                    target_position, ..
                }
                | GameEvent::HealthRestored {
                    target_position, ..
                } => sight.sees(map, *target_position),
                GameEvent::EntityDied { position, .. } => sight.sees(map, *position),
                _ => true,
            })
            .cloned()
            .collect();
        self.world.resource_mut::<Events>().0 = pending;
        self.world.resource_mut::<Events>().0.extend(filtered);
        CommandResult::Applied
    }

    fn descend(&mut self) {
        let current_floor = self.world.resource::<Floor>().0;
        if current_floor >= self.config.floor_count {
            self.world.resource_mut::<Turn>().0 += 1;
            self.world.resource_mut::<Phase>().0 = GamePhase::Victory;
            self.world
                .resource_mut::<Events>()
                .0
                .push(GameEvent::Victory);
            return;
        }

        let next_floor = current_floor + 1;
        let (map, start, _) = generation::generate_floor(self.config, next_floor);
        let mut spawn_positions: Vec<_> = map
            .tiles()
            .iter()
            .enumerate()
            .filter_map(|(index, tile)| {
                (*tile == Tile::Floor).then_some(Position {
                    x: (index % usize::from(map.width())) as i32,
                    y: (index / usize::from(map.width())) as i32,
                })
            })
            .filter(|position| *position != start)
            .collect();
        spawn_positions.sort_unstable_by_key(|position| {
            std::cmp::Reverse(position.x.abs_diff(start.x) + position.y.abs_diff(start.y))
        });

        let requested_actors: usize = self
            .content
            .spawns()
            .iter()
            .map(|spawn| spawn.count as usize)
            .sum();
        let requested_items: usize = self
            .content
            .item_spawns()
            .iter()
            .map(|spawn| spawn.count as usize)
            .sum();
        if spawn_positions.len() < requested_actors + requested_items {
            self.world.resource_mut::<Turn>().0 += 1;
            self.world
                .resource_mut::<Events>()
                .0
                .push(GameEvent::Message("Floor generation failed.".to_owned()));
            return;
        }

        let player = self.player;
        let old_entities: Vec<_> = self
            .world
            .query::<(Entity, &Enemy)>()
            .iter(&self.world)
            .map(|(entity, _)| entity)
            .collect();
        for entity in old_entities {
            self.world.despawn(entity);
        }
        self.world.insert_resource(Map(map));
        self.world.insert_resource(Floor(next_floor));
        self.world.insert_resource(GroundItems::default());
        self.world.entity_mut(player).insert(GridPosition(start));

        let mut ids = self
            .world
            .remove_resource::<NextEntityId>()
            .expect("entity ID allocator");
        let mut actor_index = 0;
        for spawn in self.content.spawns() {
            let actor = self
                .content
                .actor(&spawn.actor_id)
                .expect("validated spawn actor");
            for _ in 0..spawn.count {
                let position = spawn_positions[actor_index];
                actor_index += 1;
                self.world.spawn((
                    Enemy,
                    Behavior(actor.behavior),
                    AttackDamage(actor.attack_damage),
                    Identity(ids.allocate()),
                    GridPosition(position),
                    Health {
                        current: actor.max_health,
                        maximum: actor.max_health,
                    },
                    Display {
                        glyph: actor.glyph,
                        name: actor.name.clone(),
                    },
                    BlocksMovement,
                ));
            }
        }

        let item_positions = spawn_positions.into_iter().skip(requested_actors);
        let ground_items = self
            .content
            .item_spawns()
            .iter()
            .flat_map(|spawn| (0..spawn.count).map(|_| spawn.item_id.clone()))
            .zip(item_positions)
            .map(|(item_id, position)| resources::GroundItem {
                id: ids.allocate(),
                item_id,
                position,
                quantity: 1,
            })
            .collect();
        self.world.insert_resource(GroundItems(ground_items));
        self.world.insert_resource(ids);
        self.world.insert_resource(Visibility::default());
        self.world.resource_mut::<Turn>().0 += 1;
        systems::update_visibility(&mut self.world);
        self.world
            .resource_mut::<Events>()
            .0
            .push(GameEvent::FloorChanged { floor: next_floor });
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
                    health: entity.get::<Health>().map(|health| HealthSnapshot {
                        current: health.current,
                        maximum: health.maximum,
                    }),
                })
            })
            .collect();
        entities.sort_unstable_by_key(|entity| entity.id.0);
        let health = self
            .world
            .get::<Health>(self.player)
            .expect("player health");
        let items = self
            .world
            .resource::<GroundItems>()
            .0
            .iter()
            .filter(|item| sight.sees(map, item.position))
            .filter_map(|item| {
                let definition = self.content.item(&item.item_id)?;
                Some(ItemSnapshot {
                    id: item.id,
                    item_id: item.item_id.clone(),
                    name: definition.name.clone(),
                    glyph: definition.glyph,
                    quantity: item.quantity,
                    position: Some(item.position),
                })
            })
            .collect();
        RenderSnapshot {
            phase: self.world.resource::<Phase>().0,
            turn: self.turn(),
            floor: self.world.resource::<Floor>().0,
            floor_count: self.config.floor_count,
            seed: self.seed(),
            map: MapSnapshot {
                width: map.width(),
                height: map.height(),
                tiles: sight.remembered.clone(),
                visible: sight.visible.clone(),
            },
            entities,
            items,
            inventory: self
                .world
                .resource::<Inventory>()
                .0
                .iter()
                .filter_map(|entry| {
                    let item = self.content.item(&entry.item_id)?;
                    Some(ItemSnapshot {
                        id: entry.id,
                        item_id: entry.item_id.clone(),
                        name: item.name.clone(),
                        glyph: item.glyph,
                        quantity: entry.quantity,
                        position: None,
                    })
                })
                .collect(),
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
