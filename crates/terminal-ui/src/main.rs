#![forbid(unsafe_code)]

use std::{collections::VecDeque, io};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use project_rogue_engine_core::{
    Command, CommandError, CommandResult, Direction, Engine, GameEvent, RenderSnapshot,
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
    let mut engine = Engine::default();
    let mut messages = VecDeque::from(["Hunt the rat. Bump into it to attack.".to_owned()]);

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
            messages.push_back("New run. Hunt the rat.".to_owned());
        }
        record_events(&mut messages, &snapshot, engine.drain_events());
        if let CommandResult::Rejected(error) = result {
            messages.push_back(
                match error {
                    CommandError::Blocked => "Blocked. No turn spent.",
                    CommandError::GameOver => "Run ended. Press r to restart or q to quit.",
                    CommandError::NoStairs => "No stairs here.",
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

fn command_for(key: KeyEvent) -> Option<Command> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Command::Move(Direction::Up)),
        KeyCode::Down | KeyCode::Char('j') => Some(Command::Move(Direction::Down)),
        KeyCode::Left | KeyCode::Char('h') => Some(Command::Move(Direction::Left)),
        KeyCode::Right | KeyCode::Char('l') => Some(Command::Move(Direction::Right)),
        KeyCode::Char('.') | KeyCode::Char(' ') => Some(Command::Wait),
        KeyCode::Char('r') => Some(Command::Restart),
        _ => None,
    }
}

fn record_events(messages: &mut VecDeque<String>, before: &RenderSnapshot, events: Vec<GameEvent>) {
    for event in events {
        let message = match event {
            GameEvent::DamageTaken {
                target,
                amount,
                remaining_hp,
            } => {
                let name = before
                    .entities
                    .iter()
                    .find(|entity| entity.id == target)
                    .map_or("Entity", |entity| entity.name.as_str());
                format!("{name} takes {amount} damage ({remaining_hp} HP left).")
            }
            GameEvent::EntityDied { entity } => {
                let name = before
                    .entities
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
    use crossterm::event::KeyModifiers;

    #[test]
    fn controls_and_combat_log_use_engine_contracts() {
        assert_eq!(
            command_for(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE)),
            Some(Command::Wait)
        );
        assert_eq!(
            command_for(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
            Some(Command::Restart)
        );
        let before = Engine::default().snapshot();
        let rat = before
            .entities
            .iter()
            .find(|entity| entity.glyph == 'r')
            .unwrap();
        let mut messages = VecDeque::new();
        record_events(
            &mut messages,
            &before,
            vec![
                GameEvent::DamageTaken {
                    target: rat.id,
                    amount: 1,
                    remaining_hp: 0,
                },
                GameEvent::EntityDied { entity: rat.id },
            ],
        );
        assert_eq!(messages, ["Rat takes 1 damage (0 HP left).", "Rat died."]);
    }
}
