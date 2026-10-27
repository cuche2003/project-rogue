#![forbid(unsafe_code)]

use std::{collections::VecDeque, fs, io, path::Path};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use project_rogue_engine_core::{
    Command, CommandError, CommandResult, ContentPack, Direction, Engine, EntitySnapshot,
    GameConfig, GameEvent,
};

mod view;

const LOG_LIMIT: usize = 100;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal) -> io::Result<()> {
    let mut engine = load_engine()?;
    let mut messages =
        VecDeque::from(["Explore, find >, then press > on stairs to win.".to_owned()]);

    loop {
        let snapshot = engine.snapshot();
        terminal.draw(|frame| view::draw(frame, &snapshot, &messages))?;

        let Event::Key(key) = event::read()? else {
            // Redraw after resize; no polling or idle simulation ticks.
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
            return Ok(());
        }
        let Some(command) = command_for(key) else {
            continue;
        };
        let result = engine.dispatch(command);
        if command == Command::Restart {
            messages.clear();
            messages.push_back("Restarted same seed. Find > and descend.".to_owned());
        }
        record_events(&mut messages, &snapshot.entities, engine.drain_events());
        if let CommandResult::Rejected(error) = result {
            messages.push_back(
                match error {
                    CommandError::Blocked => "Blocked. No turn spent.",
                    CommandError::GameOver => "Run ended. Press r to restart or q to quit.",
                    CommandError::NoStairs => "No stairs here. Find > and stand on it.",
                    CommandError::InvalidTarget => "Invalid target.",
                }
                .to_owned(),
            );
        }
        while messages.len() > LOG_LIMIT {
            messages.pop_front();
        }
    }
}

fn load_engine() -> io::Result<Engine> {
    load_engine_from(Path::new("mods"))
}

fn load_engine_from(mods_dir: &Path) -> io::Result<Engine> {
    let mut content = ContentPack::core();
    if mods_dir.is_dir() {
        let mut paths: Vec<_> = fs::read_dir(mods_dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "ron"))
            .collect();
        paths.sort();
        for path in paths {
            let source = fs::read_to_string(&path)?;
            let addon = ContentPack::from_ron(&source).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{}: {error}", path.display()),
                )
            })?;
            content = content.compose(&addon).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{}: {error}", path.display()),
                )
            })?;
        }
    }
    Engine::with_content(GameConfig::default(), content)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn command_for(key: KeyEvent) -> Option<Command> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Command::Move(Direction::Up)),
        KeyCode::Down | KeyCode::Char('j') => Some(Command::Move(Direction::Down)),
        KeyCode::Left | KeyCode::Char('h') => Some(Command::Move(Direction::Left)),
        KeyCode::Right | KeyCode::Char('l') => Some(Command::Move(Direction::Right)),
        KeyCode::Char('.') | KeyCode::Char(' ') => Some(Command::Wait),
        KeyCode::Char('>') => Some(Command::Descend),
        KeyCode::Char('r') => Some(Command::Restart),
        _ => None,
    }
}

fn record_events(
    messages: &mut VecDeque<String>,
    before: &[EntitySnapshot],
    events: Vec<GameEvent>,
) {
    for event in events {
        let message = match event {
            GameEvent::DamageTaken {
                target,
                amount,
                remaining_hp,
            } => {
                let name = before
                    .iter()
                    .find(|entity| entity.id == target)
                    .map_or("Entity", |entity| entity.name.as_str());
                format!("{name} takes {amount} damage ({remaining_hp} HP left).")
            }
            GameEvent::EntityDied { entity } => {
                let name = before
                    .iter()
                    .find(|entry| entry.id == entity)
                    .map_or("Entity", |entry| entry.name.as_str());
                format!("{name} died.")
            }
            GameEvent::Defeat => "Defeat. Press r to restart.".to_owned(),
            GameEvent::Victory => "Victory! Press r to restart.".to_owned(),
            GameEvent::Message(text) => text,
            GameEvent::EntityMoved { .. } | GameEvent::TurnStarted { .. } => continue,
        };
        messages.push_back(message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_example_filesystem_mod() {
        let engine = load_engine_from(Path::new("../../mods")).unwrap();
        assert!(engine.world().iter_entities().any(|entity| {
            entity
                .get::<project_rogue_engine_core::Display>()
                .is_some_and(|display| display.name == "Brute" && display.glyph == 'B')
        }));
    }
    use crossterm::event::KeyModifiers;
    use project_rogue_engine_core::{EntityId, Position};

    #[test]
    fn controls_map_to_engine_commands() {
        for (code, command) in [
            (KeyCode::Up, Command::Move(Direction::Up)),
            (KeyCode::Char('k'), Command::Move(Direction::Up)),
            (KeyCode::Down, Command::Move(Direction::Down)),
            (KeyCode::Char('j'), Command::Move(Direction::Down)),
            (KeyCode::Left, Command::Move(Direction::Left)),
            (KeyCode::Char('h'), Command::Move(Direction::Left)),
            (KeyCode::Right, Command::Move(Direction::Right)),
            (KeyCode::Char('l'), Command::Move(Direction::Right)),
            (KeyCode::Char('.'), Command::Wait),
            (KeyCode::Char(' '), Command::Wait),
            (KeyCode::Char('>'), Command::Descend),
            (KeyCode::Char('r'), Command::Restart),
        ] {
            assert_eq!(
                command_for(KeyEvent::new(code, KeyModifiers::NONE)),
                Some(command)
            );
        }
        assert_eq!(
            command_for(KeyEvent::new(KeyCode::Char('>'), KeyModifiers::SHIFT)),
            Some(Command::Descend)
        );
        assert_eq!(
            command_for(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
            None
        );
    }

    #[test]
    fn event_log_uses_snapshot_names_and_handles_unseen_entities() {
        let rat_id = EntityId(42);
        let before = vec![EntitySnapshot {
            id: rat_id,
            position: Position { x: 7, y: 5 },
            glyph: 'r',
            name: "Rat".to_owned(),
        }];
        let mut messages = VecDeque::new();
        record_events(
            &mut messages,
            &before,
            vec![
                GameEvent::TurnStarted { turn: 1 },
                GameEvent::DamageTaken {
                    target: rat_id,
                    amount: 1,
                    remaining_hp: 0,
                },
                GameEvent::EntityDied { entity: rat_id },
                GameEvent::EntityDied {
                    entity: EntityId(99),
                },
                GameEvent::Message("No stairs here.".to_owned()),
                GameEvent::Victory,
                GameEvent::Defeat,
            ],
        );
        assert_eq!(
            messages,
            [
                "Rat takes 1 damage (0 HP left).",
                "Rat died.",
                "Entity died.",
                "No stairs here.",
                "Victory! Press r to restart.",
                "Defeat. Press r to restart.",
            ]
        );
    }
}
