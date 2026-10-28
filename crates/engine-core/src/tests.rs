use super::*;

fn fixture() -> Engine {
    let mut engine = Engine::default();
    let extras: Vec<_> = engine
        .world
        .iter_entities()
        .filter(|entity| {
            entity
                .get::<Identity>()
                .is_some_and(|identity| identity.0 != EntityId(0) && identity.0 != EntityId(1))
        })
        .map(|entity| entity.id())
        .collect();
    for entity in extras {
        engine.world.despawn(entity);
    }
    let mut map = GameMap::new(12, 12, Tile::Floor);
    for x in 0..12 {
        for y in 0..12 {
            if x == 0 || y == 0 || x == 11 || y == 11 {
                map.set_tile(Position { x, y }, Tile::Wall);
            }
        }
    }
    map.set_tile(Position { x: 9, y: 9 }, Tile::StairsDown);
    engine.world.insert_resource(Map(map));
    engine
        .world
        .entity_mut(engine.player)
        .insert(GridPosition(Position { x: 2, y: 2 }));
    let enemy = enemy(&mut engine);
    engine
        .world
        .entity_mut(enemy)
        .insert(GridPosition(Position { x: 9, y: 2 }));
    engine.world.insert_resource(Visibility::default());
    systems::update_visibility(&mut engine.world);
    engine
}

fn enemy(engine: &mut Engine) -> Entity {
    engine
        .world
        .query::<(Entity, &Enemy)>()
        .iter(&engine.world)
        .next()
        .unwrap()
        .0
}

fn adjacent_enemy_game() -> Engine {
    let mut engine = fixture();
    let enemy = enemy(&mut engine);
    engine
        .world
        .entity_mut(enemy)
        .insert(GridPosition(Position { x: 3, y: 2 }));
    systems::update_visibility(&mut engine.world);
    engine
}

#[test]
fn rejects_invalid_dimensions_before_allocation() {
    for (map_width, map_height) in [(0, 0), (7, 20), (40, 257), (u16::MAX, u16::MAX)] {
        assert!(
            Engine::try_new(GameConfig {
                map_width,
                map_height,
                ..GameConfig::default()
            })
            .is_err()
        );
    }
}

#[test]
fn accepted_turn_moves_enemy_once_and_rejected_commands_change_nothing() {
    let mut engine = fixture();
    assert_eq!(
        engine.dispatch(Command::Move(Direction::Right)),
        CommandResult::Applied
    );
    assert_eq!(engine.turn(), 1);
    assert_eq!(engine.player_position(), Position { x: 3, y: 2 });
    let enemy = enemy(&mut engine);
    assert_eq!(
        engine.world.get::<GridPosition>(enemy).unwrap().0,
        Position { x: 8, y: 2 }
    );
    assert_eq!(engine.drain_events().len(), 2);
    engine
        .world
        .resource_mut::<Map>()
        .0
        .set_tile(Position { x: 3, y: 1 }, Tile::Wall);
    systems::update_visibility(&mut engine.world);
    let initial = engine.snapshot();
    for command in [Command::Move(Direction::Up), Command::Descend] {
        assert!(matches!(
            engine.dispatch(command),
            CommandResult::Rejected(_)
        ));
        assert_eq!(engine.snapshot(), initial);
        assert!(engine.drain_events().is_empty());
    }
}

#[test]
fn player_damage_cleanup_precedes_enemy_response_and_death_frees_tile() {
    let mut engine = adjacent_enemy_game();
    for hp in [2, 1] {
        assert_eq!(
            engine.dispatch(Command::Move(Direction::Right)),
            CommandResult::Applied
        );
        assert_eq!(engine.player_position(), Position { x: 2, y: 2 });
        assert_eq!(
            engine.drain_events(),
            vec![
                GameEvent::DamageTaken {
                    target: EntityId(1),
                    amount: 1,
                    remaining_hp: hp
                },
                GameEvent::DamageTaken {
                    target: EntityId(0),
                    amount: 1,
                    remaining_hp: 7 + hp
                },
            ]
        );
    }
    engine.dispatch(Command::Move(Direction::Right));
    assert_eq!(
        engine.drain_events(),
        vec![
            GameEvent::DamageTaken {
                target: EntityId(1),
                amount: 1,
                remaining_hp: 0
            },
            GameEvent::EntityDied {
                entity: EntityId(1)
            },
        ]
    );
    assert_eq!(
        engine.world.get::<Health>(engine.player).unwrap().current,
        8
    );
    assert_eq!(engine.turn(), 3);
    engine.dispatch(Command::Move(Direction::Right));
    assert_eq!(engine.player_position(), Position { x: 3, y: 2 });
}

#[test]
fn defeat_stops_remaining_enemies_and_rejects_future_turns() {
    let mut engine = adjacent_enemy_game();
    engine.world.spawn((
        Enemy,
        Behavior(BehaviorKind::Chase),
        AttackDamage(1),
        Identity(EntityId(2)),
        GridPosition(Position { x: 2, y: 3 }),
        Health {
            current: 3,
            maximum: 3,
        },
        BlocksMovement,
    ));
    engine
        .world
        .get_mut::<Health>(engine.player)
        .unwrap()
        .current = 1;
    engine.dispatch(Command::Wait);
    assert_eq!(
        engine.drain_events(),
        vec![
            GameEvent::DamageTaken {
                target: EntityId(0),
                amount: 1,
                remaining_hp: 0
            },
            GameEvent::EntityDied {
                entity: EntityId(0)
            },
            GameEvent::Defeat,
        ]
    );
    let final_state = engine.snapshot();
    assert_eq!(final_state.phase, GamePhase::Defeat);
    assert_eq!(final_state.player.health.current, 0);
    assert_eq!(final_state.turn, 1);
    assert_eq!(
        engine.dispatch(Command::Wait),
        CommandResult::Rejected(CommandError::GameOver)
    );
    assert_eq!(engine.snapshot(), final_state);
    assert!(engine.drain_events().is_empty());
}

#[test]
fn enemy_pathfinding_goes_around_walls_and_respects_occupants() {
    let mut engine = fixture();
    let enemy = enemy(&mut engine);
    engine
        .world
        .entity_mut(enemy)
        .insert(GridPosition(Position { x: 4, y: 2 }));
    engine
        .world
        .resource_mut::<Map>()
        .0
        .set_tile(Position { x: 3, y: 2 }, Tile::Wall);
    engine.dispatch(Command::Wait);
    assert_eq!(
        engine.world.get::<GridPosition>(enemy).unwrap().0,
        Position { x: 4, y: 1 }
    );
    for position in [Position { x: 3, y: 1 }, Position { x: 5, y: 1 }] {
        engine.world.spawn((BlocksMovement, GridPosition(position)));
    }
    engine.drain_events();
    engine.dispatch(Command::Wait);
    assert_eq!(
        engine.world.get::<GridPosition>(enemy).unwrap().0,
        Position { x: 4, y: 1 }
    );
    assert!(engine.drain_events().is_empty());
}

#[test]
fn enemy_order_uses_stable_identity_not_spawn_order() {
    let mut engine = fixture();
    for (id, position) in [(3, Position { x: 5, y: 4 }), (2, Position { x: 4, y: 5 })] {
        engine.world.spawn((
            Enemy,
            Behavior(BehaviorKind::Chase),
            AttackDamage(1),
            Identity(EntityId(id)),
            GridPosition(position),
            BlocksMovement,
        ));
    }
    engine.dispatch(Command::Wait);
    let ids: Vec<_> = engine
        .drain_events()
        .into_iter()
        .map(|event| {
            let GameEvent::EntityMoved { entity, .. } = event else {
                panic!("expected movement")
            };
            entity
        })
        .collect();
    assert_eq!(ids, vec![EntityId(1), EntityId(2), EntityId(3)]);
}

#[test]
fn stairs_are_walkable_and_descent_wins_without_enemy_retaliation() {
    let mut engine = fixture();
    engine
        .world
        .resource_mut::<Map>()
        .0
        .set_tile(Position { x: 3, y: 2 }, Tile::StairsDown);
    assert_eq!(
        engine.dispatch(Command::Move(Direction::Right)),
        CommandResult::Applied
    );
    assert_eq!(engine.player_position(), Position { x: 3, y: 2 });
    let enemy = enemy(&mut engine);
    engine
        .world
        .entity_mut(enemy)
        .insert(GridPosition(Position { x: 4, y: 2 }));
    engine
        .world
        .get_mut::<Health>(engine.player)
        .unwrap()
        .current = 1;
    engine.drain_events();
    assert_eq!(engine.dispatch(Command::Descend), CommandResult::Applied);
    assert_eq!(
        engine.drain_events(),
        vec![GameEvent::FloorChanged { floor: 2 }]
    );
    assert_eq!(engine.snapshot().phase, GamePhase::Playing);
    assert_eq!(engine.snapshot().floor, 2);
    assert_eq!(engine.snapshot().player.health.current, 1);
    assert!(
        engine
            .snapshot()
            .map
            .tiles
            .iter()
            .filter(|tile| tile.is_some())
            .count()
            < 12 * 12
    );
    assert_eq!(engine.turn(), 2);
    assert_eq!(engine.dispatch(Command::Wait), CommandResult::Applied);
}

#[test]
fn restart_reproduces_seed_dimensions_state_and_empty_events() {
    let config = GameConfig {
        seed: 42,
        map_width: 32,
        map_height: 18,
        floor_count: 3,
    };
    let mut engine = Engine::new(config);
    let initial = engine.snapshot();
    engine.dispatch(Command::Wait);
    engine.dispatch(Command::Restart);
    assert_eq!(engine.snapshot(), initial);
    assert_eq!(engine.seed(), 42);
    assert!(engine.drain_events().is_empty());
}

#[test]
fn hidden_entities_and_unexplored_terrain_do_not_leak_into_snapshot() {
    let mut engine = fixture();
    for y in 1..11 {
        engine
            .world
            .resource_mut::<Map>()
            .0
            .set_tile(Position { x: 5, y }, Tile::Wall);
    }
    engine.world.insert_resource(Visibility::default());
    systems::update_visibility(&mut engine.world);
    let snapshot = engine.snapshot();
    assert_eq!(snapshot.entities.len(), 1);
    assert_eq!(snapshot.map.tiles[2 * 12 + 9], None);
    assert!(!snapshot.map.visible[2 * 12 + 9]);
    assert_eq!(snapshot.map.tiles[2 * 12 + 5], Some(Tile::Wall));
    engine
        .world
        .entity_mut(engine.player)
        .insert(GridPosition(Position { x: 8, y: 2 }));
    systems::update_visibility(&mut engine.world);
    assert_eq!(engine.snapshot().entities.len(), 2);
    engine
        .world
        .entity_mut(engine.player)
        .insert(GridPosition(Position { x: 2, y: 2 }));
    systems::update_visibility(&mut engine.world);
    let snapshot = engine.snapshot();
    assert_eq!(snapshot.entities.len(), 1);
    assert_eq!(snapshot.map.tiles[2 * 12 + 9], Some(Tile::Floor));
    assert!(!snapshot.map.visible[2 * 12 + 9]);
}

#[test]
fn replay_playback_reproduces_snapshot_and_events() {
    let mut engine = Engine::new(GameConfig {
        seed: 17,
        floor_count: 2,
        ..GameConfig::default()
    });
    engine.start_recording();
    for command in [
        Command::Move(Direction::Right),
        Command::Wait,
        Command::Pickup,
        Command::UseItem(0),
        Command::Restart,
        Command::Move(Direction::Down),
    ] {
        engine.dispatch(command);
    }
    let replay = engine.stop_recording().expect("recording started");
    let mut playback = replay.playback().unwrap();
    assert_eq!(engine.snapshot(), playback.snapshot());
    assert_eq!(engine.drain_events(), playback.drain_events());
    assert_eq!(replay.commands().len(), 6);
    let encoded = replay.to_ron().unwrap();
    let decoded = Replay::from_ron(&encoded).unwrap();
    let decoded_playback = decoded.playback().unwrap();
    assert_eq!(decoded_playback.snapshot(), playback.snapshot());
    replay.verify().unwrap();
    assert!(matches!(
        Replay::from_ron(
            "(version: 2, config: (seed: 0, map_width: 8, map_height: 8, floor_count: 1), content: (actors: [], spawns: [], items: [], item_spawns: []), commands: [])"
        ),
        Err(ReplayError::UnsupportedVersion(2))
    ));
}

#[test]
fn replay_records_rejected_commands_in_order() {
    let mut engine = Engine::default();
    engine.start_recording();
    assert_eq!(
        engine.dispatch(Command::UseItem(8)),
        CommandResult::Rejected(CommandError::NoItem)
    );
    let replay = engine.stop_recording().unwrap();
    assert_eq!(replay.commands(), &[Command::UseItem(8)]);
}

#[test]
fn same_seed_and_commands_reproduce_snapshot_and_events() {
    for seed in 0..16 {
        let mut first = Engine::new(GameConfig {
            seed,
            ..GameConfig::default()
        });
        let mut second = Engine::new(GameConfig {
            seed,
            ..GameConfig::default()
        });
        for command in [
            Command::Move(Direction::Right),
            Command::Move(Direction::Down),
            Command::Wait,
            Command::Descend,
        ] {
            assert_eq!(first.dispatch(command), second.dispatch(command));
            assert_eq!(first.snapshot(), second.snapshot());
            assert_eq!(first.drain_events(), second.drain_events());
        }
    }
}
