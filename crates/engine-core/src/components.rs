use bevy_ecs::component::Component;

use crate::{EntityId, Position};

/// Stable game identity, independent of Bevy entity allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct Identity(pub EntityId);

/// Marks the player entity.
#[derive(Debug, Default, Component)]
pub struct Player;

/// Marks an enemy entity.
#[derive(Debug, Default, Component)]
pub struct Enemy;

/// Position of an entity on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct GridPosition(pub Position);

/// Damage dealt by an actor's basic attack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct AttackDamage(pub u32);

/// AI behavior selected by actor content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct Behavior(pub crate::BehaviorKind);

/// Entity health.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct Health {
    pub current: u32,
    pub maximum: u32,
}

/// Entity blocks movement into its tile.
#[derive(Debug, Default, Component)]
pub struct BlocksMovement;

/// Presentation hints without terminal or renderer types.
#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct Display {
    pub glyph: char,
    pub name: String,
}
