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

/// Validated actor content supplied to a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentPack {
    actors: Vec<ActorDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentError {
    Parse(String),
    InvalidId(String),
    DuplicateId(String),
    EmptyName(String),
    InvalidStats(String),
    MissingActor(String),
}

impl std::fmt::Display for ContentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "content parse failed: {error}"),
            Self::InvalidId(id) => write!(f, "invalid actor ID `{id}`; expected namespace:name"),
            Self::DuplicateId(id) => write!(f, "duplicate actor ID `{id}`"),
            Self::EmptyName(id) => write!(f, "actor `{id}` has empty name"),
            Self::InvalidStats(id) => write!(f, "actor `{id}` has zero health or attack damage"),
            Self::MissingActor(id) => write!(f, "required actor `{id}` is missing"),
        }
    }
}

impl std::error::Error for ContentError {}

impl ContentPack {
    /// Builds and validates content supplied by a host or mod loader.
    pub fn new(actors: Vec<ActorDefinition>) -> Result<Self, ContentError> {
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
        Ok(Self { actors })
    }

    /// Parses RON without touching the filesystem. Hosts decide where files come from.
    pub fn from_ron(source: &str) -> Result<Self, ContentError> {
        let actors: Vec<ActorDefinition> =
            ron::from_str(source).map_err(|error| ContentError::Parse(error.to_string()))?;
        Self::new(actors)
    }

    /// Default content shipped with the game.
    #[must_use]
    pub fn core() -> Self {
        Self::from_ron(include_str!("../content/core.ron")).expect("built-in content is valid")
    }

    #[must_use]
    pub fn actor(&self, id: &str) -> Option<&ActorDefinition> {
        self.actors.iter().find(|actor| actor.id == id)
    }

    #[must_use]
    pub fn actors(&self) -> &[ActorDefinition] {
        &self.actors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"[
        (id: "mod:rat", name: "Mod Rat", glyph: 'r', max_health: 9, attack_damage: 2, behavior: Chase),
        (id: "mod:guard", name: "Guard", glyph: 'G', max_health: 4, attack_damage: 1, behavior: Guard),
    ]"#;

    #[test]
    fn parses_and_validates_mod_content() {
        let pack = ContentPack::from_ron(VALID).unwrap();
        assert_eq!(pack.actor("mod:rat").unwrap().max_health, 9);
        assert_eq!(
            pack.actor("mod:guard").unwrap().behavior,
            BehaviorKind::Guard
        );
    }

    #[test]
    fn rejects_duplicate_ids_bad_stats_and_unknown_behavior() {
        let duplicate = format!(
            "[{VALID}, (id: \"mod:rat\", name: \"x\", glyph: 'x', max_health: 1, attack_damage: 1, behavior: Passive)]"
        );
        assert!(matches!(
            ContentPack::from_ron(&duplicate),
            Err(ContentError::Parse(_)) | Err(ContentError::DuplicateId(_))
        ));
        let bad_stats = r#"[(id: "mod:bad", name: "Bad", glyph: 'x', max_health: 0, attack_damage: 1, behavior: Passive)]"#;
        assert!(matches!(
            ContentPack::from_ron(bad_stats),
            Err(ContentError::InvalidStats(_))
        ));
        let unknown = r#"[(id: "mod:bad", name: "Bad", glyph: 'x', max_health: 1, attack_damage: 1, behavior: Fly)]"#;
        assert!(matches!(
            ContentPack::from_ron(unknown),
            Err(ContentError::Parse(_))
        ));
    }
}
