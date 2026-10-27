use project_rogue_engine_core::{
    Command, Engine, GameEvent, GridPosition, Identity, Map, Position, Tile,
};

#[test]
fn hidden_enemy_moves_are_not_reported_and_pending_events_are_preserved() {
    let mut engine = Engine::default();
    let extras: Vec<_> = engine
        .world()
        .iter_entities()
        .filter(|entity| {
            entity
                .get::<Identity>()
                .is_some_and(|identity| identity.0.0 > 1)
        })
        .map(|entity| entity.id())
        .collect();
    for entity in extras {
        engine.world_mut().despawn(entity);
    }
    let rat = engine
        .world()
        .iter_entities()
        .find_map(|entity| (entity.get::<Identity>()?.0.0 == 1).then_some(entity.id()))
        .unwrap();
    // Wall separates actors, but open ends allow the rat to path around it out of sight.
    let mut map = project_rogue_engine_core::GameMap::new(40, 20, Tile::Floor);
    for y in 0..19 {
        map.set_tile(Position { x: 10, y }, Tile::Wall);
    }
    engine.world_mut().insert_resource(Map(map));
    let player = engine
        .world()
        .iter_entities()
        .find_map(|entity| (entity.get::<Identity>()?.0.0 == 0).then_some(entity.id()))
        .unwrap();
    engine
        .world_mut()
        .entity_mut(player)
        .insert(GridPosition(Position { x: 2, y: 2 }));
    engine
        .world_mut()
        .entity_mut(rat)
        .insert(GridPosition(Position { x: 18, y: 2 }));
    engine.dispatch(Command::Wait);
    assert_eq!(engine.snapshot().entities.len(), 1);
    assert!(
        engine
            .drain_events()
            .iter()
            .all(|event| !matches!(event, GameEvent::EntityMoved { .. }))
    );
    // Normal player movement is queued across turns even if callers delay draining.
    engine.dispatch(Command::Move(project_rogue_engine_core::Direction::Right));
    engine.dispatch(Command::Move(project_rogue_engine_core::Direction::Down));
    let events = engine.drain_events();
    assert_eq!(events.iter().filter(|event| matches!(event, GameEvent::EntityMoved { entity, .. } if *entity == engine.player_id())).count(), 2);
}
