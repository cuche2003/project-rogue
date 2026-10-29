use project_rogue_engine_core::{
    Command, CommandResult, ContentPack, Direction, Engine, GameConfig, GridPosition, Health,
    Identity, Map, Position, Tile,
};

const MOD_CONTENT: &str = r#"(
    actors: [
        (id: "core:player", name: "Player", glyph: '@', max_health: 10, attack_damage: 1, defense: 0, behavior: Passive),
        (id: "mod:brute", name: "Brute", glyph: 'B', max_health: 8, attack_damage: 3, behavior: Guard),
    ],
    spawns: [(actor_id: "mod:brute", count: 1)],
)"#;

const DEFENSE_CONTENT: &str = r#"(
    actors: [
        (id: "core:player", name: "Player", glyph: '@', max_health: 10, attack_damage: 1, defense: 0, behavior: Passive),
        (id: "mod:guard", name: "Guard", glyph: 'G', max_health: 8, attack_damage: 3, defense: 2, behavior: Guard),
    ],
    spawns: [(actor_id: "mod:guard", count: 1)],
)"#;

fn engine(seed: u64) -> Engine {
    engine_with_content(seed, MOD_CONTENT)
}

fn engine_with_content(seed: u64, source: &str) -> Engine {
    Engine::with_content(
        GameConfig {
            seed,
            ..GameConfig::default()
        },
        ContentPack::from_ron(source).unwrap(),
    )
    .unwrap()
}

#[test]
fn custom_actor_spawns_with_mod_stats_and_behavior() {
    let mut engine = engine(7);
    let player_position = engine.player_position();
    let enemy = engine
        .world()
        .iter_entities()
        .find(|entity| entity.get::<Identity>().is_some_and(|id| id.0.0 == 1))
        .unwrap()
        .id();
    let enemy_position = Position {
        x: player_position.x + 1,
        y: player_position.y,
    };
    engine
        .world_mut()
        .resource_mut::<Map>()
        .0
        .set_tile(enemy_position, Tile::Floor);
    engine
        .world_mut()
        .entity_mut(enemy)
        .insert(GridPosition(enemy_position));

    let brute = engine
        .snapshot()
        .entities
        .into_iter()
        .find(|entity| entity.id.0 == 1)
        .unwrap();
    assert_eq!((brute.name.as_str(), brute.glyph), ("Brute", 'B'));
    assert_eq!(engine.world().get::<Health>(enemy).unwrap().maximum, 8);

    assert_eq!(
        engine.dispatch(Command::Move(Direction::Right)),
        CommandResult::Applied
    );
    let events = engine.drain_events();
    assert!(events.iter().any(
        |event| matches!(event, project_rogue_engine_core::GameEvent::AttackResolved {
            target, attack_power: 1, defense: 0, hit: true, ..
        } if target.0 == 1)
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        project_rogue_engine_core::GameEvent::DamageTaken {
            target, amount: 1, remaining_hp: 7, ..
        } if target.0 == 1
    )));
    assert_eq!(engine.world().get::<Health>(enemy).unwrap().current, 7);
}

#[test]
fn custom_actor_defense_causes_deterministic_miss() {
    let mut engine = engine_with_content(7, DEFENSE_CONTENT);
    let player_position = engine.player_position();
    let enemy = engine
        .world()
        .iter_entities()
        .find(|entity| entity.get::<Identity>().is_some_and(|id| id.0.0 == 1))
        .unwrap()
        .id();
    let enemy_position = Position {
        x: player_position.x + 1,
        y: player_position.y,
    };
    engine
        .world_mut()
        .resource_mut::<Map>()
        .0
        .set_tile(enemy_position, Tile::Floor);
    engine
        .world_mut()
        .entity_mut(enemy)
        .insert(GridPosition(enemy_position));

    assert_eq!(
        engine.dispatch(Command::Move(Direction::Right)),
        CommandResult::Applied
    );
    let events = engine.drain_events();
    assert!(events.iter().any(|event| matches!(
        event,
        project_rogue_engine_core::GameEvent::AttackResolved {
            target, attack_power: 1, defense: 2, hit: false, ..
        } if target.0 == 1
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        project_rogue_engine_core::GameEvent::DamageTaken { target, .. } if target.0 == 1
    )));
    assert_eq!(engine.world().get::<Health>(enemy).unwrap().current, 8);
}

#[test]
fn same_seed_and_same_mod_content_spawn_identical_entities() {
    let first = engine(99).snapshot();
    let second = engine(99).snapshot();
    assert_eq!(first, second);
}
