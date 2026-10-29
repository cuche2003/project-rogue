use crate::{EntityId, ItemEffect, Position};

/// Event describing a simulation result without frontend-specific formatting.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum GameEvent {
    TurnStarted {
        turn: u64,
    },
    EntityMoved {
        entity: EntityId,
        from: Position,
        to: Position,
    },
    DamageTaken {
        source: Option<EntityId>,
        target: EntityId,
        target_position: Position,
        amount: u32,
        remaining_hp: u32,
    },
    HealthRestored {
        source: Option<EntityId>,
        target: EntityId,
        target_position: Position,
        amount: u32,
        current_hp: u32,
    },
    EntityDied {
        entity: EntityId,
        position: Position,
    },
    ItemPickedUp {
        item: EntityId,
    },
    ItemUsed {
        item: EntityId,
        item_id: String,
        item_name: String,
        effect: ItemEffect,
        target: EntityId,
        amount: u32,
    },
    EffectLimitReached {
        limit: usize,
    },
    FloorChanged {
        floor: u16,
    },
    Message(String),
    Victory,
    Defeat,
}
