use bevy_ecs::prelude::Resource;

use crate::{GameMap, Position, Tile};

const SIGHT_RADIUS: i32 = 8;

/// Player knowledge: remembered terrain never exposes unseen map changes.
#[derive(Default, Resource)]
pub(crate) struct Visibility {
    pub visible: Vec<bool>,
    pub remembered: Vec<Option<Tile>>,
}

impl Visibility {
    pub fn update(&mut self, map: &GameMap, origin: Position) {
        let count = map.tiles().len();
        self.visible.resize(count, false);
        self.visible.fill(false);
        self.remembered.resize(count, None);
        // ponytail: bounded ray-per-cell FOV; use shadowcasting if sight radius or actor count grows.
        for y in (origin.y - SIGHT_RADIUS).max(0)
            ..=(origin.y + SIGHT_RADIUS).min(i32::from(map.height()) - 1)
        {
            for x in (origin.x - SIGHT_RADIUS).max(0)
                ..=(origin.x + SIGHT_RADIUS).min(i32::from(map.width()) - 1)
            {
                let target = Position { x, y };
                if (x - origin.x).pow(2) + (y - origin.y).pow(2) > SIGHT_RADIUS.pow(2)
                    || !line_of_sight(map, origin, target)
                {
                    continue;
                }
                let index = y as usize * usize::from(map.width()) + x as usize;
                self.visible[index] = true;
                self.remembered[index] = map.tile(target);
            }
        }
    }

    pub fn sees(&self, map: &GameMap, position: Position) -> bool {
        if map.tile(position).is_none() {
            return false;
        }
        self.visible[position.y as usize * usize::from(map.width()) + position.x as usize]
    }
}

fn line_of_sight(map: &GameMap, mut from: Position, to: Position) -> bool {
    let dx = (to.x - from.x).abs();
    let dy = -(to.y - from.y).abs();
    let sx = (to.x - from.x).signum();
    let sy = (to.y - from.y).signum();
    let mut error = dx + dy;
    while from != to {
        let previous = from;
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            from.x += sx;
        }
        if twice <= dx {
            error += dx;
            from.y += sy;
        }
        if from.x != previous.x
            && from.y != previous.y
            && map.tile(Position {
                x: from.x,
                y: previous.y,
            }) == Some(Tile::Wall)
            && map.tile(Position {
                x: previous.x,
                y: from.y,
            }) == Some(Tile::Wall)
        {
            return false;
        }
        if from == to {
            return true;
        }
        if map.tile(from) == Some(Tile::Wall) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walls_occlude_and_memory_does_not_leak_unseen_changes() {
        let mut map = GameMap::new(24, 8, Tile::Floor);
        for y in 0..8 {
            map.set_tile(Position { x: 3, y }, Tile::Wall);
        }
        let mut sight = Visibility::default();
        sight.update(&map, Position { x: 1, y: 1 });
        assert!(sight.sees(&map, Position { x: 3, y: 1 }));
        assert!(!sight.sees(&map, Position { x: 4, y: 1 }));
        assert_eq!(sight.remembered[4 + 24], None);
        map.set_tile(Position { x: 1, y: 1 }, Tile::Wall);
        sight.update(&map, Position { x: 20, y: 1 });
        assert!(!sight.sees(&map, Position { x: 1, y: 1 }));
        assert_eq!(sight.remembered[1 + 24], Some(Tile::Floor));
    }

    #[test]
    fn cannot_see_through_closed_diagonal_corner() {
        let mut map = GameMap::new(8, 8, Tile::Floor);
        map.set_tile(Position { x: 2, y: 1 }, Tile::Wall);
        map.set_tile(Position { x: 1, y: 2 }, Tile::Wall);
        let mut sight = Visibility::default();
        sight.update(&map, Position { x: 1, y: 1 });
        assert!(!sight.sees(&map, Position { x: 2, y: 2 }));
    }
}
