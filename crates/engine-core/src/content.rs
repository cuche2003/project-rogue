use serde::Deserialize;

/// Built-in AI behavior selected by actor content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum BehaviorKind {
    Passive,
    Chase,
    Guard,
}

/// Stable namespaced content ID, such as `core:rat`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ActorDefinition {
    pub id: String,
    pub name: String,
    pub glyph: char,
    pub max_health: u32,
    pub attack_damage: u32,
    pub behavior: BehaviorKind,
}

/// Deterministic actor spawn request.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SpawnDefinition {
    pub actor_id: String,
    pub count: u32,
}

#[derive(Debug, Deserialize)]
struct ContentFile {
    actors: Vec<ActorDefinition>,
    #[serde(default)]
    spawns: Vec<SpawnDefinition>,
}

/// Validated content supplied to a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentPack {
    actors: Vec<ActorDefinition>,
    spawns: Vec<SpawnDefinition>,
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
        }
    }
}

impl std::error::Error for ContentError {}

impl ContentPack {
    /// Builds and validates actor and spawn content supplied by a host or mod loader.
    pub fn new(
        actors: Vec<ActorDefinition>,
        spawns: Vec<SpawnDefinition>,
    ) -> Result<Self, ContentError> {
        let mut ids = std::collections::HashSet::with_capacity(actors.len());
        for actor in &actors {
            let mut parts = actor.id.split(':');
            let namespace = parts.next();
            let name = parts.next();
            if actor.id.trim() != actor.id
                || namespace.is_none_or(str::is_empty)
                || name.is_none_or(str::is_empty)
                || parts.next().is_some()
            {
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

        Ok(Self { actors, spawns })
    }

    /// Parses RON without touching the filesystem. Hosts decide where files come from.
    pub fn from_ron(source: &str) -> Result<Self, ContentError> {
        let file: ContentFile =
            ron::from_str(source).map_err(|error| ContentError::Parse(error.to_string()))?;
        Self::new(file.actors, file.spawns)
    }

    /// Default content shipped with the game.
    #[must_use]
    pub fn core() -> Self {
        Self::from_ron(include_str!("../content/core.ron")).expect("built-in content is valid")
    }

    /// Combines base content with one mod pack. Actor IDs are immutable across packs.
    pub fn compose(&self, addon: &Self) -> Result<Self, ContentError> {
        let mut actors = self.actors.clone();
        actors.extend(addon.actors.iter().cloned());
        let mut spawns = self.spawns.clone();
        spawns.extend(addon.spawns.iter().cloned());
        Self::new(actors, spawns)
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
    pub fn actors(&self) -> &[ActorDefinition] {
        &self.actors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"(
        actors: [
            (id: "mod:rat", name: "Mod Rat", glyph: 'r', max_health: 9, attack_damage: 2, behavior: Chase),
            (id: "mod:guard", name: "Guard", glyph: 'G', max_health: 4, attack_damage: 1, behavior: Guard),
        ],
        spawns: [(actor_id: "mod:rat", count: 2), (actor_id: "mod:guard", count: 1)],
    )"#;

    #[test]
    fn parses_and_validates_mod_content() {
        let pack = ContentPack::from_ron(VALID).unwrap();
        assert_eq!(pack.actor("mod:rat").unwrap().max_health, 9);
        assert_eq!(
            pack.actor("mod:guard").unwrap().behavior,
            BehaviorKind::Guard
        );
        assert_eq!(pack.spawns()[0].count, 2);
    }

    #[test]
    fn composes_addon_and_rejects_conflicts() {
        let base = ContentPack::from_ron(VALID).unwrap();
        let addon = ContentPack::from_ron(
            r#"(
                actors: [(id: "addon:brute", name: "Brute", glyph: 'B', max_health: 8, attack_damage: 3, behavior: Chase)],
                spawns: [(actor_id: "addon:brute", count: 1)],
            )"#,
        )
        .unwrap();
        let composed = base.compose(&addon).unwrap();
        assert_eq!(composed.actor("addon:brute").unwrap().attack_damage, 3);
        assert_eq!(composed.spawns().len(), 3);

        let conflict = ContentPack::from_ron(
            r#"(
                actors: [(id: "mod:rat", name: "Other", glyph: 'x', max_health: 1, attack_damage: 1, behavior: Passive)],
                spawns: [],
            )"#,
        )
        .unwrap();
        assert!(matches!(
            base.compose(&conflict),
            Err(ContentError::DuplicateId(_))
        ));
    }

    #[test]
    fn rejects_bad_spawns_and_unknown_behavior() {
        let unknown = r#"(
            actors: [(id: "mod:bad", name: "Bad", glyph: 'x', max_health: 1, attack_damage: 1, behavior: Fly)],
            spawns: [],
        )"#;
        assert!(matches!(
            ContentPack::from_ron(unknown),
            Err(ContentError::Parse(_))
        ));

        let unknown_actor = r#"(
            actors: [(id: "mod:bad", name: "Bad", glyph: 'x', max_health: 1, attack_damage: 1, behavior: Passive)],
            spawns: [(actor_id: "mod:missing", count: 1)],
        )"#;
        assert!(matches!(
            ContentPack::from_ron(unknown_actor),
            Err(ContentError::UnknownSpawnActor(_))
        ));
    }
}
