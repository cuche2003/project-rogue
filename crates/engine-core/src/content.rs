use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorKind {
    Passive,
    Chase,
    Guard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorDefinition {
    pub id: String,
    pub name: String,
    pub glyph: char,
    pub max_health: u32,
    pub attack_damage: u32,
    #[serde(default)]
    pub defense: u32,
    pub behavior: BehaviorKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnDefinition {
    pub actor_id: String,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemEffect {
    Heal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDefinition {
    pub id: String,
    pub name: String,
    pub glyph: char,
    pub stack_limit: u32,
    pub effect: ItemEffect,
    pub power: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemSpawnDefinition {
    pub item_id: String,
    pub count: u32,
}

#[derive(Debug, Deserialize)]
struct ContentFile {
    actors: Vec<ActorDefinition>,
    #[serde(default)]
    spawns: Vec<SpawnDefinition>,
    #[serde(default)]
    items: Vec<ItemDefinition>,
    #[serde(default)]
    item_spawns: Vec<ItemSpawnDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ContentFile")]
pub struct ContentPack {
    actors: Vec<ActorDefinition>,
    spawns: Vec<SpawnDefinition>,
    items: Vec<ItemDefinition>,
    item_spawns: Vec<ItemSpawnDefinition>,
}

impl TryFrom<ContentFile> for ContentPack {
    type Error = ContentError;

    fn try_from(file: ContentFile) -> Result<Self, Self::Error> {
        Self::new(file.actors, file.spawns, file.items, file.item_spawns)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentError {
    Parse(String),
    InvalidId(String),
    DuplicateId(String),
    EmptyName(String),
    InvalidStats(String),
    DuplicateSpawn(String),
    UnknownSpawnActor(String),
    EmptySpawn(String),
    DuplicateItemId(String),
    InvalidItemId(String),
    EmptyItemName(String),
    InvalidItemStats(String),
    DuplicateItemSpawn(String),
    UnknownItem(String),
    EmptyItemSpawn(String),
}

impl std::fmt::Display for ContentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "content parse failed: {error}"),
            Self::InvalidId(id) => write!(f, "invalid actor ID `{id}`; expected namespace:name"),
            Self::DuplicateId(id) => write!(f, "duplicate actor ID `{id}`"),
            Self::EmptyName(id) => write!(f, "actor `{id}` has empty name"),
            Self::InvalidStats(id) => write!(f, "actor `{id}` has zero health or attack damage"),
            Self::DuplicateSpawn(id) => write!(f, "duplicate spawn definition for `{id}`"),
            Self::UnknownSpawnActor(id) => write!(f, "spawn references unknown actor `{id}`"),
            Self::EmptySpawn(id) => write!(f, "spawn `{id}` has zero count"),
            Self::DuplicateItemId(id) => write!(f, "duplicate item ID `{id}`"),
            Self::InvalidItemId(id) => write!(f, "invalid item ID `{id}`; expected namespace:name"),
            Self::EmptyItemName(id) => write!(f, "item `{id}` has empty name"),
            Self::InvalidItemStats(id) => write!(f, "item `{id}` has invalid stack limit or power"),
            Self::DuplicateItemSpawn(id) => write!(f, "duplicate item spawn for `{id}`"),
            Self::UnknownItem(id) => write!(f, "item spawn references unknown item `{id}`"),
            Self::EmptyItemSpawn(id) => write!(f, "item spawn `{id}` has zero count"),
        }
    }
}

impl std::error::Error for ContentError {}

fn valid_id(id: &str) -> bool {
    let mut parts = id.split(':');
    id.trim() == id
        && parts.next().is_some_and(|part| !part.is_empty())
        && parts.next().is_some_and(|part| !part.is_empty())
        && parts.next().is_none()
}

impl ContentPack {
    pub fn new(
        actors: Vec<ActorDefinition>,
        spawns: Vec<SpawnDefinition>,
        items: Vec<ItemDefinition>,
        item_spawns: Vec<ItemSpawnDefinition>,
    ) -> Result<Self, ContentError> {
        let mut ids = std::collections::HashSet::with_capacity(actors.len());
        for actor in &actors {
            if !valid_id(&actor.id) {
                return Err(ContentError::InvalidId(actor.id.clone()));
            }
            if !ids.insert(actor.id.clone()) {
                return Err(ContentError::DuplicateId(actor.id.clone()));
            }
            if actor.name.trim().is_empty() {
                return Err(ContentError::EmptyName(actor.id.clone()));
            }
            if actor.max_health == 0 || actor.attack_damage == 0 {
                return Err(ContentError::InvalidStats(actor.id.clone()));
            }
        }

        let mut spawn_ids = std::collections::HashSet::with_capacity(spawns.len());
        for spawn in &spawns {
            if spawn.count == 0 {
                return Err(ContentError::EmptySpawn(spawn.actor_id.clone()));
            }
            if !spawn_ids.insert(spawn.actor_id.clone()) {
                return Err(ContentError::DuplicateSpawn(spawn.actor_id.clone()));
            }
            if !ids.contains(&spawn.actor_id) {
                return Err(ContentError::UnknownSpawnActor(spawn.actor_id.clone()));
            }
        }

        let mut item_ids = std::collections::HashSet::with_capacity(items.len());
        for item in &items {
            if !valid_id(&item.id) {
                return Err(ContentError::InvalidItemId(item.id.clone()));
            }
            if !item_ids.insert(item.id.clone()) {
                return Err(ContentError::DuplicateItemId(item.id.clone()));
            }
            if item.name.trim().is_empty() {
                return Err(ContentError::EmptyItemName(item.id.clone()));
            }
            if item.stack_limit == 0 || item.power == 0 {
                return Err(ContentError::InvalidItemStats(item.id.clone()));
            }
        }

        let mut item_spawn_ids = std::collections::HashSet::with_capacity(item_spawns.len());
        for spawn in &item_spawns {
            if spawn.count == 0 {
                return Err(ContentError::EmptyItemSpawn(spawn.item_id.clone()));
            }
            if !item_spawn_ids.insert(spawn.item_id.clone()) {
                return Err(ContentError::DuplicateItemSpawn(spawn.item_id.clone()));
            }
            if !item_ids.contains(&spawn.item_id) {
                return Err(ContentError::UnknownItem(spawn.item_id.clone()));
            }
        }

        Ok(Self {
            actors,
            spawns,
            items,
            item_spawns,
        })
    }

    pub fn from_ron(source: &str) -> Result<Self, ContentError> {
        let file: ContentFile =
            ron::from_str(source).map_err(|error| ContentError::Parse(error.to_string()))?;
        Self::new(file.actors, file.spawns, file.items, file.item_spawns)
    }

    #[must_use]
    pub fn core() -> Self {
        Self::from_ron(include_str!("../content/core.ron")).expect("built-in content is valid")
    }

    pub fn compose(&self, addon: &Self) -> Result<Self, ContentError> {
        let mut actors = self.actors.clone();
        actors.extend(addon.actors.iter().cloned());
        let mut spawns = self.spawns.clone();
        spawns.extend(addon.spawns.iter().cloned());
        let mut items = self.items.clone();
        items.extend(addon.items.iter().cloned());
        let mut item_spawns = self.item_spawns.clone();
        item_spawns.extend(addon.item_spawns.iter().cloned());
        Self::new(actors, spawns, items, item_spawns)
    }

    #[must_use]
    pub fn actor(&self, id: &str) -> Option<&ActorDefinition> {
        self.actors.iter().find(|actor| actor.id == id)
    }
    #[must_use]
    pub fn spawns(&self) -> &[SpawnDefinition] {
        &self.spawns
    }
    #[must_use]
    pub fn item(&self, id: &str) -> Option<&ItemDefinition> {
        self.items.iter().find(|item| item.id == id)
    }
    #[must_use]
    pub fn item_spawns(&self) -> &[ItemSpawnDefinition] {
        &self.item_spawns
    }
    #[must_use]
    pub fn actors(&self) -> &[ActorDefinition] {
        &self.actors
    }
    #[must_use]
    pub fn items(&self) -> &[ItemDefinition] {
        &self.items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_items_and_rejects_unknown_item_spawns() {
        let valid = r#"(
            actors: [], spawns: [],
            items: [(id: "core:potion", name: "Potion", glyph: '!', stack_limit: 5, effect: Heal, power: 5)],
            item_spawns: [(item_id: "core:potion", count: 2)],
        )"#;
        let pack = ContentPack::from_ron(valid).unwrap();
        assert_eq!(pack.item("core:potion").unwrap().power, 5);
        let invalid = r#"(
            actors: [], spawns: [], items: [],
            item_spawns: [(item_id: "core:missing", count: 1)],
        )"#;
        assert!(matches!(
            ContentPack::from_ron(invalid),
            Err(ContentError::UnknownItem(_))
        ));
    }
}
