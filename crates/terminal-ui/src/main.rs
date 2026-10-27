#![forbid(unsafe_code)]

use std::{io, time::Duration};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use project_rogue_engine_core::{Direction, Engine};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction as LayoutDirection, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut engine = Engine::new();

    loop {
        terminal.draw(|frame| draw(frame, &engine))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => break,
            KeyCode::Up | KeyCode::Char('k') => engine.step(Direction::Up),
            KeyCode::Down | KeyCode::Char('j') => engine.step(Direction::Down),
            KeyCode::Left | KeyCode::Char('h') => engine.step(Direction::Left),
            KeyCode::Right | KeyCode::Char('l') => engine.step(Direction::Right),
            _ => {}
        }
    }

    Ok(())
}

fn draw(frame: &mut Frame, engine: &Engine) {
    let [header_area, map_area, footer_area] = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .areas(frame.area());

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "PROJECT ROGUE",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  |  turn "),
        Span::styled(
            engine.turn().to_string(),
            Style::default().fg(Color::Yellow),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL).title(" Status "));
    frame.render_widget(header, header_area);

    let position = engine.player_position();
    let map_text = format!(
        "\n\n                         @\n\n                   position ({}, {})",
        position.x, position.y
    );
    let map = Paragraph::new(map_text)
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::ALL).title(" Dungeon "));
    frame.render_widget(map, map_area);

    let footer = Paragraph::new("Arrows / hjkl move   |   q / Esc quit")
        .block(Block::default().borders(Borders::ALL).title(" Commands "));
    frame.render_widget(footer, footer_area);
}
