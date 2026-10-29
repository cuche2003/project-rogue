#![forbid(unsafe_code)]

use std::{
    collections::VecDeque,
    env, fs,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use project_rogue_engine_core::{
    Command, CommandError, CommandResult, ContentPack, Direction, Engine, EntitySnapshot,
    GameConfig, GameEvent, Replay,
};

mod view;

const LOG_LIMIT: usize = 100;

fn main() -> io::Result<()> {
    let mode = run_mode(env::args_os().skip(1))?;
    if let RunMode::Verify(path) = &mode {
        let replay = load_replay(path)?;
        replay.verify().map_err(invalid_data)?;
        println!(
            "Verification succeeded: {} commands reproduced identical results, snapshots, and events.",
            replay.commands().len()
        );
        return Ok(());
    }
    if matches!(mode, RunMode::Agent) {
        return run_agent();
    }
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, mode);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, mode: RunMode) -> io::Result<()> {
    let mut playback = None;
    let mut engine = match &mode {
        RunMode::Replay(path) => {
            let replay = load_replay(path)?;
            playback = Some(ReplayPlayback {
                commands: replay.commands().to_vec(),
                ..ReplayPlayback::default()
            });
            replay.start().map_err(invalid_data)?
        }
        RunMode::Verify(_) | RunMode::Agent => {
            return Err(invalid_data("mode must run headlessly"));
        }
        RunMode::Normal | RunMode::Record(_) => {
            let mut engine = load_engine()?;
            if matches!(mode, RunMode::Record(_)) {
                engine.start_recording();
            }
            engine
        }
    };
    let mut messages = VecDeque::from([if playback.is_some() {
        "Replay: Space/Enter to step, p to play/pause, q to quit.".to_owned()
    } else {
        "Explore, find >, then press > on stairs to win.".to_owned()
    }]);
    let mut help = false;

    loop {
        let snapshot = engine.snapshot();
        terminal.draw(|frame| {
            view::draw(
                frame,
                &snapshot,
                &messages,
                playback
                    .as_ref()
                    .map(|replay| (replay.index, replay.commands.len(), replay.playing)),
                help,
            )
        })?;

        let input = read_input(
            playback.as_ref().is_some_and(|replay| replay.playing),
            event::poll,
            event::read,
        )?;
        if help {
            if let Some(Event::Key(key)) = input
                && key.kind == KeyEventKind::Press
                && matches!(key.code, KeyCode::Char('?') | KeyCode::Esc)
            {
                help = false;
            }
            continue;
        }
        if let Some(Event::Key(key)) = &input
            && key.kind == KeyEventKind::Press
            && key.code == KeyCode::Char('?')
        {
            help = true;
            continue;
        }
        if let Some(replay) = &mut playback {
            if replay.handle_input(input, &mut engine, &mut messages) {
                return Ok(());
            }
            continue;
        }
        let Some(Event::Key(key)) = input else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if key.code == KeyCode::Char('?') {
            help = true;
            continue;
        }
        if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
            if let (RunMode::Record(path), Some(replay)) = (&mode, engine.stop_recording()) {
                fs::write(path, replay.to_ron().map_err(invalid_data)?)?;
            }
            return Ok(());
        }
        if let Some(command) = command_for(key) {
            dispatch_command(&mut engine, command, &mut messages);
        }
    }
}

fn read_input(
    playing: bool,
    mut poll: impl FnMut(Duration) -> io::Result<bool>,
    mut read: impl FnMut() -> io::Result<Event>,
) -> io::Result<Option<Event>> {
    if playing && !poll(Duration::from_millis(80))? {
        return Ok(None);
    }
    read().map(Some)
}

#[derive(Default)]
struct ReplayPlayback {
    commands: Vec<Command>,
    index: usize,
    playing: bool,
}

impl ReplayPlayback {
    // None means the autoplay poll timed out; input always takes priority over stepping.
    fn handle_input(
        &mut self,
        input: Option<Event>,
        engine: &mut Engine,
        messages: &mut VecDeque<String>,
    ) -> bool {
        match input {
            Some(Event::Key(key)) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return true,
                KeyCode::Char('p') => {
                    self.playing = !self.playing && self.index < self.commands.len();
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    self.playing = false;
                    self.step(engine, messages);
                }
                _ => {}
            },
            None if self.playing => self.step(engine, messages),
            _ => {}
        }
        false
    }

    fn step(&mut self, engine: &mut Engine, messages: &mut VecDeque<String>) {
        if let Some(&command) = self.commands.get(self.index) {
            dispatch_command(engine, command, messages);
            self.index += 1;
        }
        if self.index == self.commands.len() {
            self.playing = false;
        }
    }
}

fn dispatch_command(engine: &mut Engine, command: Command, messages: &mut VecDeque<String>) {
    let before = engine.snapshot();
    let result = engine.dispatch(command);
    if command == Command::Restart {
        messages.clear();
        messages.push_back("Restarted same seed. Find > and descend.".to_owned());
    }
    record_events(messages, &before.entities, engine.drain_events());
    if let CommandResult::Rejected(error) = result {
        messages.push_back(
            match error {
                CommandError::Blocked => "Blocked. No turn spent.",
                CommandError::GameOver => "Run ended. Press r to restart or q to quit.",
                CommandError::NoStairs => "No stairs here. Find > and stand on it.",
                CommandError::InvalidTarget => "Invalid target.",
                CommandError::NoItem => "No item here or in that inventory slot.",
                CommandError::InventoryFull => "Inventory full.",
            }
            .to_owned(),
        );
    }
    while messages.len() > LOG_LIMIT {
        messages.pop_front();
    }
}

enum RunMode {
    Normal,
    Agent,
    Record(PathBuf),
    Replay(PathBuf),
    Verify(PathBuf),
}

fn run_mode(mut args: impl Iterator<Item = std::ffi::OsString>) -> io::Result<RunMode> {
    let Some(flag) = args.next() else {
        return Ok(RunMode::Normal);
    };
    if flag == "--agent" {
        if let Some(extra) = args.next() {
            return Err(invalid_data(format!("unexpected argument: {extra:?}")));
        }
        return Ok(RunMode::Agent);
    }
    let mode = match flag.to_str() {
        Some("--record") => RunMode::Record,
        Some("--replay") => RunMode::Replay,
        Some("--verify") => RunMode::Verify,
        _ => return Err(invalid_data(format!("unknown argument: {flag:?}"))),
    };
    let path = args
        .next()
        .filter(|path| !path.is_empty() && !path.to_string_lossy().starts_with("--"))
        .ok_or_else(|| invalid_data(format!("{} requires a path", flag.to_string_lossy())))?;
    if let Some(extra) = args.next() {
        return Err(invalid_data(format!("unexpected argument: {extra:?}")));
    }
    Ok(mode(PathBuf::from(path)))
}

fn run_agent() -> io::Result<()> {
    let mut engine = load_engine()?;
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in BufReader::new(stdin.lock()).lines() {
        let line = line?;
        let command = match parse_agent_command(&line) {
            Ok(command) => command,
            Err(error) => {
                writeln!(stdout, "{{\"error\":{}}}", json_string(&error))?;
                stdout.flush()?;
                continue;
            }
        };
        let Some(command) = command else {
            continue;
        };
        if command == AgentCommand::Quit {
            writeln!(stdout, "{{\"ok\":true,\"quit\":true}}")?;
            break;
        }
        let AgentCommand::Game(command) = command else {
            unreachable!("quit handled above");
        };
        let result = engine.dispatch(command);
        let events = engine.drain_events();
        writeln!(
            stdout,
            "{{\"result\":{},\"events\":{},\"snapshot\":{}}}",
            json_value(&result),
            json_value(&events),
            json_value(&engine.snapshot())
        )?;
        stdout.flush()?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentCommand {
    Game(Command),
    Quit,
}

fn parse_agent_command(line: &str) -> Result<Option<AgentCommand>, String> {
    let mut words = line.split_whitespace();
    let Some(action) = words.next() else {
        return Ok(None);
    };
    let command = match action {
        "move" => {
            let direction = match words.next() {
                Some("up") => Direction::Up,
                Some("down") => Direction::Down,
                Some("left") => Direction::Left,
                Some("right") => Direction::Right,
                Some(direction) => return Err(format!("invalid direction: {direction}")),
                None => return Err("move requires a direction".to_owned()),
            };
            AgentCommand::Game(Command::Move(direction))
        }
        "wait" => AgentCommand::Game(Command::Wait),
        "pickup" => AgentCommand::Game(Command::Pickup),
        "descend" => AgentCommand::Game(Command::Descend),
        "restart" => AgentCommand::Game(Command::Restart),
        "use" => {
            let slot = words
                .next()
                .ok_or_else(|| "use requires an inventory slot".to_owned())?
                .parse::<u8>()
                .map_err(|_| "use slot must be an integer from 0 to 8".to_owned())?;
            if slot >= 9 {
                return Err("use slot must be an integer from 0 to 8".to_owned());
            }
            AgentCommand::Game(Command::UseItem(slot))
        }
        "quit" => AgentCommand::Quit,
        _ => return Err(format!("unknown command: {action}")),
    };
    if words.next().is_some() {
        return Err("unexpected extra arguments".to_owned());
    }
    Ok(Some(command))
}

fn json_value<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("agent response is serializable")
}

fn json_string(value: &str) -> String {
    json_value(&value)
}

fn invalid_data(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

fn load_replay(path: &Path) -> io::Result<Replay> {
    Replay::from_ron(&fs::read_to_string(path)?).map_err(invalid_data)
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
        KeyCode::Char('g') => Some(Command::Pickup),
        KeyCode::Char(c @ '1'..='9') => Some(Command::UseItem(c as u8 - b'1')),
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
                ..
            } => {
                let name = before
                    .iter()
                    .find(|entity| entity.id == target)
                    .map_or("Entity", |entity| entity.name.as_str());
                format!("{name} takes {amount} damage ({remaining_hp} HP left).")
            }
            GameEvent::HealthRestored {
                target,
                amount,
                current_hp,
                ..
            } => {
                let name = before
                    .iter()
                    .find(|entity| entity.id == target)
                    .map_or("Entity", |entity| entity.name.as_str());
                format!("{name} restores {amount} HP ({current_hp} HP).")
            }
            GameEvent::EntityDied { entity, .. } => {
                let name = before
                    .iter()
                    .find(|entry| entry.id == entity)
                    .map_or("Entity", |entry| entry.name.as_str());
                format!("{name} died.")
            }
            GameEvent::Defeat => "Defeat. Press r to restart.".to_owned(),
            GameEvent::Victory => "Victory! Press r to restart.".to_owned(),
            GameEvent::ItemPickedUp { .. } => "Picked up item.".to_owned(),
            GameEvent::ItemUsed {
                item_name, amount, ..
            } => {
                format!("Used {item_name} ({amount} HP).")
            }
            GameEvent::EffectLimitReached { limit } => {
                format!("Effect processing stopped at {limit} operations.")
            }
            GameEvent::FloorChanged { floor } => format!("Entered floor {floor}."),
            GameEvent::Message(text) => text,
            GameEvent::EntityMoved { .. } | GameEvent::TurnStarted { .. } => continue,
        };
        messages.push_back(message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn replay_starts_paused_and_steps_exactly_one_command_per_press() {
        let mut replay = ReplayPlayback {
            commands: vec![Command::Wait, Command::Descend, Command::Restart],
            ..ReplayPlayback::default()
        };
        let mut engine = Engine::default();
        let initial = engine.snapshot();
        let mut messages = VecDeque::new();
        replay.handle_input(None, &mut engine, &mut messages);
        assert!(!replay.playing);
        assert_eq!(engine.snapshot(), initial);
        let mut expected = Engine::default();
        for (index, code) in [KeyCode::Char(' '), KeyCode::Enter, KeyCode::Enter]
            .into_iter()
            .enumerate()
        {
            assert!(!replay.handle_input(Some(key(code)), &mut engine, &mut messages));
            expected.dispatch(replay.commands[index]);
            assert_eq!(replay.index, index + 1);
            assert_eq!(engine.snapshot(), expected.snapshot());
            assert!(!replay.playing);
        }
        assert_eq!(engine.snapshot(), initial);
        replay.handle_input(Some(key(KeyCode::Enter)), &mut engine, &mut messages);
        assert_eq!(replay.index, 3);
    }

    #[test]
    fn replay_ignores_gameplay_keys_and_repeat_release_events() {
        let mut replay = ReplayPlayback {
            commands: vec![Command::Wait],
            ..ReplayPlayback::default()
        };
        let mut engine = Engine::default();
        let initial = engine.snapshot();
        let mut messages = VecDeque::new();
        for code in [
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Char('h'),
            KeyCode::Char('j'),
            KeyCode::Char('k'),
            KeyCode::Char('l'),
            KeyCode::Char('.'),
            KeyCode::Char('g'),
            KeyCode::Char('1'),
            KeyCode::Char('9'),
            KeyCode::Char('>'),
            KeyCode::Char('r'),
        ] {
            replay.handle_input(Some(key(code)), &mut engine, &mut messages);
        }
        for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
            let mut event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
            event.kind = kind;
            replay.handle_input(Some(Event::Key(event)), &mut engine, &mut messages);
        }
        assert_eq!(replay.index, 0);
        assert_eq!(engine.snapshot(), initial);
    }

    #[test]
    fn autoplay_polls_and_handles_pause_quit_before_dispatch() {
        let mut replay = ReplayPlayback {
            commands: vec![Command::Wait; 3],
            ..ReplayPlayback::default()
        };
        let mut engine = Engine::default();
        let mut messages = VecDeque::new();
        replay.handle_input(Some(key(KeyCode::Char('p'))), &mut engine, &mut messages);
        assert!(replay.playing);
        let tick = read_input(
            replay.playing,
            |timeout| {
                assert_eq!(timeout, Duration::from_millis(80));
                Ok(false)
            },
            || panic!("timeout must not read"),
        )
        .unwrap();
        replay.handle_input(tick, &mut engine, &mut messages);
        assert_eq!(replay.index, 1);
        for code in [KeyCode::Char('p'), KeyCode::Char('q'), KeyCode::Esc] {
            replay.playing = true;
            let input = read_input(replay.playing, |_| Ok(true), || Ok(key(code))).unwrap();
            let quit = replay.handle_input(input, &mut engine, &mut messages);
            assert_eq!(quit, code != KeyCode::Char('p'));
            assert_eq!(replay.index, 1);
            if !quit {
                assert!(!replay.playing);
                replay.handle_input(None, &mut engine, &mut messages);
                assert_eq!(replay.index, 1);
            }
        }
        let input = read_input(
            false,
            |_| panic!("paused replay must block on read"),
            || Ok(key(KeyCode::Enter)),
        )
        .unwrap();
        replay.handle_input(input, &mut engine, &mut messages);
        assert_eq!(replay.index, 2);
        replay.handle_input(Some(key(KeyCode::Char('p'))), &mut engine, &mut messages);
        replay.handle_input(None, &mut engine, &mut messages);
        assert_eq!(replay.index, 3);
        assert!(!replay.playing);
        replay.handle_input(Some(key(KeyCode::Char('p'))), &mut engine, &mut messages);
        assert!(!replay.playing);
    }

    #[test]
    fn empty_replay_stays_complete_without_dispatch() {
        let mut replay = ReplayPlayback::default();
        let mut engine = Engine::default();
        let initial = engine.snapshot();
        let mut messages = VecDeque::new();
        for input in [
            Some(key(KeyCode::Char('p'))),
            None,
            Some(key(KeyCode::Enter)),
            Some(key(KeyCode::Char(' '))),
        ] {
            replay.handle_input(input, &mut engine, &mut messages);
            assert_eq!(replay.index, 0);
            assert!(!replay.playing);
        }
        assert_eq!(engine.snapshot(), initial);
    }

    #[test]
    fn loads_example_filesystem_mod() {
        let mods = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
        let engine = load_engine_from(&mods).unwrap();
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
            health: None,
        }];
        let mut messages = VecDeque::new();
        record_events(
            &mut messages,
            &before,
            vec![
                GameEvent::TurnStarted { turn: 1 },
                GameEvent::DamageTaken {
                    source: Some(EntityId(0)),
                    target: rat_id,
                    target_position: Position { x: 7, y: 5 },
                    amount: 1,
                    remaining_hp: 0,
                },
                GameEvent::EntityDied {
                    entity: rat_id,
                    position: Position { x: 7, y: 5 },
                },
                GameEvent::EntityDied {
                    entity: EntityId(99),
                    position: Position { x: 20, y: 20 },
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
