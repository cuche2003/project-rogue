//! Platform-independent game simulation for Project Rogue.

use bevy_ecs::prelude::{Component, Entity, Resource, World};

/// Grid coordinate used by game state.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

/// Marker for the player entity.
#[derive(Debug, Default, Component)]
pub struct Player;

/// Current simulation turn, stored as an ECS resource.
#[derive(Debug, Default, Resource)]
pub struct Turn(pub u64);

/// Direction accepted by game commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Platform-independent game state backed by Bevy ECS.
pub struct Engine {
    world: World,
    player: Entity,
}

impl Engine {
    /// Creates a new simulation in its initial state.
    #[must_use]
    pub fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(Turn::default());
        let player = world.spawn((Player, Position::default())).id();

        Self { world, player }
    }

    /// Returns the ECS world for read-only queries and resources.
    #[must_use]
    pub const fn world(&self) -> &World {
        &self.world
    }

    /// Returns the ECS world for custom systems and mod content.
    #[must_use]
    pub const fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Returns the player entity ID.
    #[must_use]
    pub const fn player_entity(&self) -> Entity {
        self.player
    }

    /// Returns the player's current position.
    #[must_use]
    pub fn player_position(&self) -> Position {
        *self
            .world
            .get::<Position>(self.player)
            .expect("engine player entity must have Position")
    }

    /// Returns the current simulation turn.
    #[must_use]
    pub fn turn(&self) -> u64 {
        self.world.resource::<Turn>().0
    }

    /// Advances the simulation by one turn in given direction.
    pub fn step(&mut self, direction: Direction) {
        let mut query = self.world.query::<&mut Position>();
        let mut position = query
            .get_mut(&mut self.world, self.player)
            .expect("engine player entity must have Position");

        match direction {
            Direction::Up => position.y -= 1,
            Direction::Down => position.y += 1,
            Direction::Left => position.x -= 1,
            Direction::Right => position.x += 1,
        }

        self.world.resource_mut::<Turn>().0 += 1;
    }

    /// Advances the simulation by one turn.
    pub fn advance_turn(&mut self) {
        self.world.resource_mut::<Turn>().0 += 1;
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Direction, Engine, Player, Position};

    #[test]
    fn step_updates_position_and_turn() {
        let mut engine = Engine::new();

        engine.step(Direction::Right);
        engine.step(Direction::Down);

        assert_eq!(engine.player_position(), Position { x: 1, y: 1 });
        assert_eq!(engine.turn(), 2);
    }

    #[test]
    fn world_exposes_ecs_components_for_frontends_and_mods() {
        let mut engine = Engine::new();
        let player = engine.player_entity();

        assert!(engine.world().get::<Player>(player).is_some());
        engine
            .world_mut()
            .entity_mut(player)
            .insert(Position { x: 4, y: 2 });
        assert_eq!(engine.player_position(), Position { x: 4, y: 2 });
    }
}
