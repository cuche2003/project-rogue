use bevy_ecs::prelude::Resource;

use crate::{GameMap, GamePhase};

/// Current simulation turn.
#[derive(Debug, Default, Resource)]
pub struct Turn(pub u64);

/// Current game phase.
#[derive(Debug, Resource)]
pub struct Phase(pub GamePhase);

/// Monotonic stable identity allocator.
#[derive(Debug, Default, Resource)]
pub struct NextEntityId(pub u64);

/// Runtime configuration for initial game state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameConfig {
    pub seed: u64,
    pub map_width: u16,
    pub map_height: u16,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            map_width: 40,
            map_height: 20,
        }
    }
}

/// Map owned by the simulation.
#[derive(Debug, Resource)]
pub struct Map(pub GameMap);
