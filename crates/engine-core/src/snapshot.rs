use crate::{EntityId, GamePhase, Position, Tile};

/// Complete UI-neutral view of current game state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSnapshot {
    pub phase: GamePhase,
    pub turn: u64,
    pub seed: u64,
    pub map: MapSnapshot,
    pub entities: Vec<EntitySnapshot>,
    pub player: PlayerSnapshot,
    pub messages: Vec<String>,
}

/// Renderable map data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapSnapshot {
    pub width: u16,
    pub height: u16,
    /// Last observed terrain, or None for unexplored cells.
    pub tiles: Vec<Option<Tile>>,
    /// Current line-of-sight visibility, in the same row-major order.
    pub visible: Vec<bool>,
}

/// Renderable entity data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntitySnapshot {
    pub id: EntityId,
    pub position: Position,
    pub glyph: char,
    pub name: String,
}

/// Player-specific view data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSnapshot {
    pub id: EntityId,
    pub position: Position,
    pub health: HealthSnapshot,
}

/// Health value exposed to a frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthSnapshot {
    pub current: u32,
    pub maximum: u32,
}
