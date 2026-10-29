use crate::{EntityId, GamePhase, Position, Tile};

/// Complete UI-neutral view of current game state.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RenderSnapshot {
    pub phase: GamePhase,
    pub turn: u64,
    pub floor: u16,
    pub floor_count: u16,
    pub seed: u64,
    pub map: MapSnapshot,
    pub entities: Vec<EntitySnapshot>,
    pub items: Vec<ItemSnapshot>,
    pub player: PlayerSnapshot,
    pub inventory: Vec<ItemSnapshot>,
    pub messages: Vec<String>,
}

/// Renderable map data.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct MapSnapshot {
    pub width: u16,
    pub height: u16,
    /// Last observed terrain, or None for unexplored cells.
    pub tiles: Vec<Option<Tile>>,
    /// Current line-of-sight visibility, in the same row-major order.
    pub visible: Vec<bool>,
}

/// Renderable entity data.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct EntitySnapshot {
    pub id: EntityId,
    pub position: Position,
    pub glyph: char,
    pub name: String,
    pub health: Option<HealthSnapshot>,
}

/// Item visible in player inventory or on the map.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ItemSnapshot {
    pub id: EntityId,
    pub item_id: String,
    pub name: String,
    pub glyph: char,
    pub quantity: u32,
    pub position: Option<Position>,
}

/// Player-specific view data.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PlayerSnapshot {
    pub id: EntityId,
    pub position: Position,
    pub health: HealthSnapshot,
}

/// Health value exposed to a frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct HealthSnapshot {
    pub current: u32,
    pub maximum: u32,
}
