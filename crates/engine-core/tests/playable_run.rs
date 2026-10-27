use std::collections::VecDeque;

use project_rogue_engine_core::{
    Command, CommandResult, ContentPack, Direction, Engine, GameConfig, GamePhase, GridPosition,
    Identity, Map, Position, Tile,
};

#[test]
fn generated_runs_can_fight_reach_stairs_win_and_restart() {
    for seed in 0..8 {
        let mut engine = Engine::with_content(
            GameConfig {
                seed,
                ..GameConfig::default()
            },
            ContentPack::from_ron(
                r#"(
                    actors: [
                        (id: "core:player", name: "Player", glyph: '@', max_health: 10, attack_damage: 1, behavior: Passive),
                        (id: "core:rat", name: "Rat", glyph: 'r', max_health: 3, attack_damage: 1, behavior: Chase),
                    ],
                    spawns: [(actor_id: "core:rat", count: 1)],
                )"#,
            )
            .unwrap(),
        )
        .unwrap();
        let initial = engine.snapshot();
        let start = engine.player_position();
        // Trusted test inspection, not frontend access: let the rat approach through corridors.
        for _ in 0..800 {
            let rat = engine
                .world()
                .iter_entities()
                .find_map(|entity| {
                    (entity.get::<Identity>()?.0.0 == 1)
                        .then(|| entity.get::<GridPosition>().unwrap().0)
                })
                .unwrap();
            if rat.x.abs_diff(start.x) + rat.y.abs_diff(start.y) == 1 {
                let direction = if rat.x > start.x {
                    Direction::Right
                } else if rat.x < start.x {
                    Direction::Left
                } else if rat.y > start.y {
                    Direction::Down
                } else {
                    Direction::Up
                };
                for _ in 0..3 {
                    assert_eq!(
                        engine.dispatch(Command::Move(direction)),
                        CommandResult::Applied
                    );
                }
                break;
            }
            engine.dispatch(Command::Wait);
        }
        assert!(
            engine
                .world()
                .iter_entities()
                .all(|entity| entity.get::<Identity>().is_none_or(|id| id.0.0 != 1))
        );
        let map = &engine.world().resource::<Map>().0;
        let width = usize::from(map.width());
        let index = |p: Position| p.y as usize * width + p.x as usize;
        let mut paths = vec![None; map.tiles().len()];
        let mut queue = VecDeque::from([start]);
        paths[index(start)] = Some((start, Direction::Up));
        let mut goal = None;
        while let Some(position) = queue.pop_front() {
            if map.tile(position) == Some(Tile::StairsDown) {
                goal = Some(position);
                break;
            }
            for (direction, dx, dy) in [
                (Direction::Up, 0, -1),
                (Direction::Down, 0, 1),
                (Direction::Left, -1, 0),
                (Direction::Right, 1, 0),
            ] {
                let to = Position {
                    x: position.x + dx,
                    y: position.y + dy,
                };
                if map.tile(to).is_some_and(Tile::is_walkable) && paths[index(to)].is_none() {
                    paths[index(to)] = Some((position, direction));
                    queue.push_back(to);
                }
            }
        }
        let mut goal = goal.unwrap();
        let mut route = Vec::new();
        while goal != start {
            let (previous, direction) = paths[index(goal)].unwrap();
            route.push(direction);
            goal = previous;
        }
        for direction in route.into_iter().rev() {
            for _ in 0..8 {
                let before = engine.player_position();
                assert_eq!(
                    engine.dispatch(Command::Move(direction)),
                    CommandResult::Applied
                );
                if engine.player_position() != before {
                    break;
                }
            }
        }
        assert_eq!(engine.dispatch(Command::Descend), CommandResult::Applied);
        assert_eq!(engine.snapshot().phase, GamePhase::Victory);
        engine.dispatch(Command::Restart);
        assert_eq!(engine.snapshot(), initial);
        assert!(engine.drain_events().is_empty());
    }
}
