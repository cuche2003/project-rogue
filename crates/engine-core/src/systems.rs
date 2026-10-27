use std::collections::VecDeque;

use bevy_ecs::prelude::*;

use crate::resources::INVENTORY_SLOTS;
use crate::{
    AttackDamage, Behavior, BlocksMovement, Command, CommandError, Direction, Enemy, GameEvent,
    GamePhase, GridPosition, Health, Identity, Map, Phase, Position, Tile, Turn,
    resources::{
        Content, Events, GroundItems, Inventory, InventoryEntry, PlayerEntity, TurnCommand,
    },
    visibility::Visibility,
};

/// Deterministic turn stages. Player damage is cleaned up before enemies act.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum TurnSet {
    Player,
    PlayerCleanup,
    Enemy,
    Cleanup,
    Outcome,
    Visibility,
}

pub(crate) fn schedule() -> Schedule {
    let mut schedule = Schedule::default();
    // bevy_ecs multi_threaded feature is disabled; chained stages run sequentially.
    schedule.configure_sets(
        (
            TurnSet::Player,
            TurnSet::PlayerCleanup,
            TurnSet::Enemy,
            TurnSet::Cleanup,
            TurnSet::Outcome,
            TurnSet::Visibility,
        )
            .chain(),
    );
    schedule.add_systems((
        player_action.in_set(TurnSet::Player),
        cleanup.in_set(TurnSet::PlayerCleanup),
        enemy_actions.in_set(TurnSet::Enemy),
        cleanup.in_set(TurnSet::Cleanup),
        outcome.in_set(TurnSet::Outcome),
        update_visibility.in_set(TurnSet::Visibility),
    ));
    schedule
}

fn destination(from: Position, direction: Direction) -> Position {
    match direction {
        Direction::Up => Position {
            y: from.y - 1,
            ..from
        },
        Direction::Down => Position {
            y: from.y + 1,
            ..from
        },
        Direction::Left => Position {
            x: from.x - 1,
            ..from
        },
        Direction::Right => Position {
            x: from.x + 1,
            ..from
        },
    }
}

pub(crate) fn validate(world: &mut World, command: Command) -> Result<(), CommandError> {
    if world.resource::<Phase>().0 != GamePhase::Playing {
        return Err(CommandError::GameOver);
    }
    let player = world.resource::<PlayerEntity>().0;
    let from = world
        .get::<GridPosition>(player)
        .expect("player position")
        .0;
    match command {
        Command::Move(direction) => {
            let to = destination(from, direction);
            if !world
                .resource::<Map>()
                .0
                .tile(to)
                .is_some_and(Tile::is_walkable)
            {
                return Err(CommandError::Blocked);
            }
            if let Some(entity) = occupant(world, to, player)
                && (world.get::<Enemy>(entity).is_none() || world.get::<Health>(entity).is_none())
            {
                return Err(CommandError::Blocked);
            }
        }
        Command::Descend if world.resource::<Map>().0.tile(from) != Some(Tile::StairsDown) => {
            return Err(CommandError::NoStairs);
        }
        Command::Pickup
            if !world
                .resource::<GroundItems>()
                .0
                .iter()
                .any(|item| item.position == from) =>
        {
            return Err(CommandError::NoItem);
        }
        Command::Pickup if !pickup_fits(world, from) => {
            return Err(CommandError::InventoryFull);
        }
        Command::UseItem(index)
            if world
                .resource::<Inventory>()
                .0
                .get(index as usize)
                .is_none() =>
        {
            return Err(CommandError::NoItem);
        }
        _ => {}
    }
    Ok(())
}

fn pickup_fits(world: &World, position: Position) -> bool {
    let ground = world
        .resource::<GroundItems>()
        .0
        .iter()
        .find(|item| item.position == position)
        .expect("validated item");
    let stack_limit = world
        .resource::<Content>()
        .0
        .item(&ground.item_id)
        .expect("validated item")
        .stack_limit;
    world.resource::<Inventory>().0.len() < INVENTORY_SLOTS
        || world
            .resource::<Inventory>()
            .0
            .iter()
            .any(|entry| entry.item_id == ground.item_id && entry.quantity < stack_limit)
}

fn occupant(world: &mut World, position: Position, actor: Entity) -> Option<Entity> {
    world
        .query::<(Entity, &GridPosition, &BlocksMovement)>()
        .iter(world)
        .find_map(|(entity, grid, _)| (entity != actor && grid.0 == position).then_some(entity))
}

fn move_actor(world: &mut World, entity: Entity, to: Position) {
    let from = world.get::<GridPosition>(entity).expect("actor position").0;
    let id = world.get::<Identity>(entity).expect("actor identity").0;
    world.entity_mut(entity).insert(GridPosition(to));
    world
        .resource_mut::<Events>()
        .0
        .push(GameEvent::EntityMoved {
            entity: id,
            from,
            to,
        });
}

fn damage(world: &mut World, attacker: Entity, target: Entity) {
    let id = world.get::<Identity>(target).expect("combat identity").0;
    let amount = world
        .get::<AttackDamage>(attacker)
        .map_or(1, |damage| damage.0);
    let remaining_hp = {
        let mut health = world.get_mut::<Health>(target).expect("combat health");
        health.current = health.current.saturating_sub(amount);
        health.current
    };
    world
        .resource_mut::<Events>()
        .0
        .push(GameEvent::DamageTaken {
            target: id,
            amount,
            remaining_hp,
        });
}

fn player_action(world: &mut World) {
    let command = world.resource::<TurnCommand>().0;
    let player = world.resource::<PlayerEntity>().0;
    let position = world
        .get::<GridPosition>(player)
        .expect("player position")
        .0;
    if command == Command::Pickup {
        let index = world
            .resource::<GroundItems>()
            .0
            .iter()
            .position(|item| item.position == position)
            .expect("validated item");
        let ground = world.resource_mut::<GroundItems>().0.remove(index);
        let stack_limit = world
            .resource::<Content>()
            .0
            .item(&ground.item_id)
            .expect("validated item")
            .stack_limit;
        let mut inventory = world.resource_mut::<Inventory>();
        if let Some(entry) = inventory
            .0
            .iter_mut()
            .find(|entry| entry.item_id == ground.item_id && entry.quantity < stack_limit)
        {
            entry.quantity = entry
                .quantity
                .saturating_add(ground.quantity)
                .min(stack_limit);
        } else {
            debug_assert!(inventory.0.len() < INVENTORY_SLOTS);
            inventory.0.push(InventoryEntry {
                id: ground.id,
                item_id: ground.item_id.clone(),
                quantity: ground.quantity.min(stack_limit),
            });
        }
        world
            .resource_mut::<Events>()
            .0
            .push(GameEvent::ItemPickedUp { item: ground.id });
        return;
    }
    if let Command::UseItem(index) = command {
        let item_id = world.resource::<Inventory>().0[index as usize]
            .item_id
            .clone();
        let item = world
            .resource::<Content>()
            .0
            .item(&item_id)
            .expect("validated item")
            .clone();
        match item.effect {
            crate::ItemEffect::Heal => {
                let mut health = world.get_mut::<Health>(player).expect("player health");
                health.current = health
                    .current
                    .saturating_add(item.power)
                    .min(health.maximum);
            }
        }
        let item_id = {
            let mut inventory = world.resource_mut::<Inventory>();
            let id = inventory.0[index as usize].id;
            inventory.0[index as usize].quantity -= 1;
            if inventory.0[index as usize].quantity == 0 {
                inventory.0.remove(index as usize);
            }
            id
        };
        world
            .resource_mut::<Events>()
            .0
            .push(GameEvent::ItemUsed { item: item_id });
        return;
    }
    let Command::Move(direction) = command else {
        return;
    };
    let to = destination(position, direction);
    if let Some(enemy) = occupant(world, to, player) {
        damage(world, player, enemy);
    } else {
        move_actor(world, player, to);
    }
}

fn cleanup(world: &mut World) {
    let player = world.resource::<PlayerEntity>().0;
    let mut dead: Vec<_> = world
        .query::<(Entity, &Identity, &Health)>()
        .iter(world)
        .filter(|(entity, _, hp)| *entity != player && hp.current == 0)
        .map(|(entity, id, _)| (id.0, entity))
        .collect();
    dead.sort_unstable_by_key(|(id, _)| id.0);
    for (id, entity) in dead {
        world.despawn(entity);
        world
            .resource_mut::<Events>()
            .0
            .push(GameEvent::EntityDied { entity: id });
    }
}

fn enemy_actions(world: &mut World) {
    // Successful descent leaves immediately; no enemy retaliation on the exit turn.
    if world.resource::<TurnCommand>().0 == Command::Descend {
        return;
    }
    let player = world.resource::<PlayerEntity>().0;
    let goal = world
        .get::<GridPosition>(player)
        .expect("player position")
        .0;
    let map = &world.resource::<Map>().0;
    let width = usize::from(map.width());
    let mut distances = vec![u32::MAX; map.tiles().len()];
    let index = |p: Position| p.y as usize * width + p.x as usize;
    distances[index(goal)] = 0;
    let mut queue = VecDeque::from([goal]);
    // One shared distance field per turn avoids a separate path search per enemy.
    while let Some(position) = queue.pop_front() {
        for direction in [
            Direction::Left,
            Direction::Up,
            Direction::Right,
            Direction::Down,
        ] {
            let next = destination(position, direction);
            if map.tile(next).is_some_and(Tile::is_walkable) && distances[index(next)] == u32::MAX {
                distances[index(next)] = distances[index(position)] + 1;
                queue.push_back(next);
            }
        }
    }
    let mut enemies: Vec<_> = world
        .query::<(Entity, &Identity, &Enemy)>()
        .iter(world)
        .map(|(entity, id, _)| (id.0, entity))
        .collect();
    enemies.sort_unstable_by_key(|(id, _)| id.0);
    for (_, enemy) in enemies {
        if world.get::<Health>(player).expect("player health").current == 0 {
            break;
        }
        let from = world.get::<GridPosition>(enemy).expect("enemy position").0;
        if from.x.abs_diff(goal.x) + from.y.abs_diff(goal.y) == 1 {
            if world
                .get::<Behavior>(enemy)
                .is_none_or(|behavior| behavior.0 != crate::BehaviorKind::Passive)
            {
                damage(world, enemy, player);
            }
            continue;
        }
        if world
            .get::<Behavior>(enemy)
            .is_some_and(|behavior| behavior.0 != crate::BehaviorKind::Chase)
        {
            continue;
        }
        // Fixed tie-break order; occupancy checked against previous enemies' moves.
        let mut candidates: Vec<_> = [
            Direction::Left,
            Direction::Up,
            Direction::Right,
            Direction::Down,
        ]
        .into_iter()
        .map(|dir| destination(from, dir))
        .filter(|to| {
            world
                .resource::<Map>()
                .0
                .tile(*to)
                .is_some_and(Tile::is_walkable)
        })
        .filter(|to| distances[index(*to)] < distances[index(from)])
        .collect();
        candidates.sort_by_key(|to| distances[index(*to)]);
        for to in candidates {
            if occupant(world, to, enemy).is_none() {
                move_actor(world, enemy, to);
                break;
            }
        }
    }
}

fn outcome(world: &mut World) {
    world.resource_mut::<Turn>().0 += 1;
    let player = world.resource::<PlayerEntity>().0;
    if world.get::<Health>(player).expect("player health").current == 0 {
        let id = world.get::<Identity>(player).expect("player identity").0;
        world.resource_mut::<Phase>().0 = GamePhase::Defeat;
        world
            .resource_mut::<Events>()
            .0
            .extend([GameEvent::EntityDied { entity: id }, GameEvent::Defeat]);
    }
}

pub(crate) fn update_visibility(world: &mut World) {
    let player = world.resource::<PlayerEntity>().0;
    let position = world
        .get::<GridPosition>(player)
        .expect("player position")
        .0;
    world.resource_scope(|world, mut sight: Mut<Visibility>| {
        sight.update(&world.resource::<Map>().0, position);
    });
}
