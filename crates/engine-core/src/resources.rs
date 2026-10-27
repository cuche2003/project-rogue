use bevy_ecs::prelude::Resource;

use crate::{Command, EntityId, GameEvent, GameMap, GamePhase, Position};

pub const INVENTORY_SLOTS: usize = 9;

/// Current simulation turn.
#[derive(Debug, Default, Resource)]
pub struct Turn(pub u64);

#[derive(Debug, Default, Resource)]
pub(crate) struct Floor(pub u16);

/// Current game phase.
#[derive(Debug, Resource)]
pub struct Phase(pub GamePhase);

/// Monotonic stable identity allocator.
#[derive(Debug, Default, Resource)]
pub struct NextEntityId(pub u64);

/// Reproducible run configuration. Dimensions must each be in 8..=256.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub struct GameConfig {
    pub seed: u64,
    pub map_width: u16,
    pub map_height: u16,
    pub floor_count: u16,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            map_width: 40,
            map_height: 20,
            floor_count: 3,
        }
    }
}

/// Map owned by the simulation.
#[derive(Debug, Resource)]
pub struct Map(pub GameMap);

#[derive(Resource)]
pub(crate) struct PlayerEntity(pub bevy_ecs::entity::Entity);

#[derive(Resource)]
pub(crate) struct TurnCommand(pub Command);

#[derive(Default, Resource)]
pub(crate) struct Events(pub Vec<GameEvent>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroundItem {
    pub id: EntityId,
    pub item_id: String,
    pub position: Position,
    pub quantity: u32,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct GroundItems(pub Vec<GroundItem>);

#[derive(Debug, Default, Resource)]
pub(crate) struct Inventory(pub Vec<InventoryEntry>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InventoryEntry {
    pub id: EntityId,
    pub item_id: String,
    pub quantity: u32,
}

#[derive(Debug, Resource)]
pub(crate) struct Content(pub(crate) crate::ContentPack);

impl NextEntityId {
    pub(crate) fn allocate(&mut self) -> EntityId {
        let id = EntityId(self.0);
        self.0 = self.0.checked_add(1).expect("entity ID space exhausted");
        id
    }
}
