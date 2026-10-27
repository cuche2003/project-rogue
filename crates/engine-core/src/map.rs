/// Coordinate in the game map. X increases east; Y increases south.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

/// Terrain occupying one map cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Floor,
    Wall,
    StairsDown,
}

impl Tile {
    /// Whether an actor can enter this terrain.
    #[must_use]
    pub const fn is_walkable(self) -> bool {
        matches!(self, Self::Floor | Self::StairsDown)
    }
}

/// Bounded map data owned by the simulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameMap {
    width: u16,
    height: u16,
    tiles: Vec<Tile>,
}

impl GameMap {
    #[must_use]
    pub fn new(width: u16, height: u16, fill: Tile) -> Self {
        Self {
            width,
            height,
            tiles: vec![fill; usize::from(width) * usize::from(height)],
        }
    }

    #[must_use]
    pub const fn width(&self) -> u16 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u16 {
        self.height
    }

    #[must_use]
    pub fn tile(&self, position: Position) -> Option<Tile> {
        self.index(position).map(|index| self.tiles[index])
    }

    pub fn set_tile(&mut self, position: Position, tile: Tile) -> bool {
        let Some(index) = self.index(position) else {
            return false;
        };
        self.tiles[index] = tile;
        true
    }

    #[must_use]
    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }

    fn index(&self, position: Position) -> Option<usize> {
        let x = u16::try_from(position.x).ok()?;
        let y = u16::try_from(position.y).ok()?;
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(usize::from(y) * usize::from(self.width) + usize::from(x))
    }
}
