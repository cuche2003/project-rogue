use rand_chacha::ChaCha8Rng;
use rand_core::{RngCore, SeedableRng};

use crate::{GameConfig, GameMap, Position, Tile};

/// Generate connected rooms inside a solid wall border. Dimensions are validated by the caller.
pub(crate) fn generate(config: GameConfig) -> (GameMap, Position, Position) {
    generate_floor(config, 1)
}

pub(crate) fn generate_floor(config: GameConfig, floor: u16) -> (GameMap, Position, Position) {
    let seed = if floor == 1 {
        config.seed
    } else {
        config.seed ^ u64::from(floor).wrapping_mul(0x9E37_79B9_7F4A_7C15)
    };
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut map = GameMap::new(config.map_width, config.map_height, Tile::Wall);
    let width = i32::from(config.map_width) - 2;
    let height = i32::from(config.map_height) - 2;
    let columns = (width / 12).max(1);
    let rows = (height / 10).max(1);
    let mut player = Position::default();
    let mut previous: Option<Position> = None;

    // Snake through cells so corridors connect nearby rooms rather than crossing the whole map.
    for row in 0..rows {
        for step in 0..columns {
            let column = if row % 2 == 0 {
                step
            } else {
                columns - 1 - step
            };
            let left = 1 + column * width / columns;
            let top = 1 + row * height / rows;
            let cell_width = (column + 1) * width / columns - column * width / columns;
            let cell_height = (row + 1) * height / rows - row * height / rows;
            // Leave one wall on the right/bottom of each cell to keep rooms separate.
            let room_width = 3 + below(&mut rng, cell_width - 3);
            let room_height = 3 + below(&mut rng, cell_height - 3);
            let x = left + below(&mut rng, cell_width - room_width);
            let y = top + below(&mut rng, cell_height - room_height);
            for room_y in y..y + room_height {
                for room_x in x..x + room_width {
                    map.set_tile(
                        Position {
                            x: room_x,
                            y: room_y,
                        },
                        Tile::Floor,
                    );
                }
            }
            let center = Position {
                x: x + room_width / 2,
                y: y + room_height / 2,
            };
            if let Some(start) = previous {
                let corner = if below(&mut rng, 2) == 0 {
                    Position {
                        x: center.x,
                        y: start.y,
                    }
                } else {
                    Position {
                        x: start.x,
                        y: center.y,
                    }
                };
                carve_line(&mut map, start, corner);
                carve_line(&mut map, corner, center);
            } else {
                player = center;
            }
            previous = Some(center);
        }
    }

    let floors: Vec<_> = map
        .tiles()
        .iter()
        .enumerate()
        .filter_map(|(index, tile)| {
            (*tile == Tile::Floor).then_some(Position {
                x: (index % usize::from(config.map_width)) as i32,
                y: (index / usize::from(config.map_width)) as i32,
            })
        })
        .collect();
    let stairs = floors
        .iter()
        .copied()
        .max_by_key(|position| distance(*position, player))
        .unwrap_or(player);
    let enemy = floors
        .iter()
        .copied()
        .max_by_key(|position| distance(*position, player).min(distance(*position, stairs)))
        .unwrap_or(player);
    map.set_tile(stairs, Tile::StairsDown);
    (map, player, enemy)
}

fn distance(a: Position, b: Position) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

fn carve_line(map: &mut GameMap, mut start: Position, end: Position) {
    let dx = (end.x - start.x).signum();
    let dy = (end.y - start.y).signum();
    map.set_tile(start, Tile::Floor);
    while start != end {
        start.x += dx;
        start.y += dy;
        map.set_tile(start, Tile::Floor);
    }
}

// Rejection sampling avoids modulo bias without needing rand's distribution layer.
fn below(rng: &mut ChaCha8Rng, upper: i32) -> i32 {
    let upper = upper as u32;
    let threshold = upper.wrapping_neg() % upper;
    loop {
        let value = rng.next_u32();
        if value >= threshold {
            return (value % upper) as i32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[test]
    fn seeds_reproduce_layouts_and_change_terrain() {
        for (map_width, map_height) in [(8, 8), (40, 20), (256, 256)] {
            let first = generate(GameConfig {
                seed: 0,
                map_width,
                map_height,
                floor_count: 1,
            });
            let mut terrain_changed = false;
            for seed in 0..64 {
                let config = GameConfig {
                    seed,
                    map_width,
                    map_height,
                    floor_count: 1,
                };
                let generated = generate(config);
                assert_eq!(generated, generate(config), "config: {config:?}");
                terrain_changed |= generated
                    .0
                    .tiles()
                    .iter()
                    .zip(first.0.tiles())
                    .any(|(a, b)| (*a == Tile::Wall) != (*b == Tile::Wall));
            }
            assert!(
                terrain_changed,
                "terrain never changed for {map_width}x{map_height}"
            );
        }
    }

    #[test]
    fn every_walkable_cell_is_reachable_with_distinct_spawns_and_wall_borders() {
        for (map_width, map_height) in [(8, 8), (40, 20), (256, 256), (8, 256), (256, 8)] {
            for seed in (0..64).chain([u64::MAX]) {
                let config = GameConfig {
                    seed,
                    map_width,
                    map_height,
                    floor_count: 1,
                };
                let (map, player, enemy) = generate(config);
                assert_eq!((map.width(), map.height()), (map_width, map_height));
                assert_ne!(player, enemy, "config: {config:?}");
                assert_eq!(map.tile(player), Some(Tile::Floor));
                assert_eq!(map.tile(enemy), Some(Tile::Floor));
                assert_eq!(
                    map.tiles()
                        .iter()
                        .filter(|tile| **tile == Tile::StairsDown)
                        .count(),
                    1
                );

                let index = |p: Position| p.y as usize * usize::from(map_width) + p.x as usize;
                let mut visited = vec![false; map.tiles().len()];
                visited[index(player)] = true;
                let mut queue = VecDeque::from([player]);
                while let Some(position) = queue.pop_front() {
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let next = Position {
                            x: position.x + dx,
                            y: position.y + dy,
                        };
                        if matches!(map.tile(next), Some(Tile::Floor | Tile::StairsDown))
                            && !visited[index(next)]
                        {
                            visited[index(next)] = true;
                            queue.push_back(next);
                        }
                    }
                }
                for y in 0..i32::from(map_height) {
                    for x in 0..i32::from(map_width) {
                        let position = Position { x, y };
                        let tile = map.tile(position).unwrap();
                        if x == 0
                            || y == 0
                            || x == i32::from(map_width) - 1
                            || y == i32::from(map_height) - 1
                        {
                            assert_eq!(tile, Tile::Wall, "border {position:?}, config: {config:?}");
                        }
                        assert_eq!(
                            visited[index(position)],
                            tile != Tile::Wall,
                            "reachability {position:?}, config: {config:?}"
                        );
                    }
                }
            }
        }
    }
}
