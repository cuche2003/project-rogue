use crate::{EntityId, Position};

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
        target: EntityId,
        amount: u32,
        remaining_hp: u32,
    },
    EntityDied {
        entity: EntityId,
    },
    ItemPickedUp {
        item: EntityId,
    },
    ItemUsed {
        item: EntityId,
    },
    FloorChanged {
        floor: u16,
    },
    Message(String),
    Victory,
    Defeat,
}
