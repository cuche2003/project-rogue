use std::collections::VecDeque;

use project_rogue_engine_core::{GamePhase, RenderSnapshot, Tile};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

const BACKGROUND: Color = Color::Rgb(16, 20, 29);
const TEXT: Color = Color::Rgb(214, 222, 235);
const MUTED: Color = Color::Rgb(112, 131, 153);
const ACCENT: Color = Color::Rgb(104, 214, 196);

pub fn draw(frame: &mut Frame, snapshot: &RenderSnapshot, messages: &VecDeque<String>) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(BACKGROUND).fg(TEXT)),
        area,
    );
    if area.width < 30 || area.height < 12 {
        frame.render_widget(
            Paragraph::new("Terminal too small.\nResize to at least 30 x 12.\nq: quit")
                .style(Style::default().fg(TEXT)),
            area,
        );
        return;
    }
    let [status, map_area, log, controls] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(5),
        Constraint::Length(2),
    ])
    .areas(area);
    let hp = snapshot.player.health;
    let phase = match snapshot.phase {
        GamePhase::Defeat => "DEFEAT - r: restart",
        GamePhase::Victory => "VICTORY - r: restart",
        GamePhase::Playing if snapshot.entities.len() == 1 => "Rat defeated! r: new run",
        GamePhase::Playing => "Hunt the rat",
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" HP {}/{} ", hp.current, hp.maximum),
                Style::default()
                    .fg(if hp.current <= 3 {
                        Color::LightRed
                    } else {
                        ACCENT
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(" Turn {} | {phase}", snapshot.turn)),
        ]))
        .block(panel(" PROJECT ROGUE ")),
        status,
    );

    let border = panel(" Dungeon | @ player  r rat  # wall ");
    let inner = border.inner(map_area);
    frame.render_widget(border, map_area);
    // Follow player when the terminal cannot fit the full map.
    let origin_x = (snapshot.player.position.x - i32::from(inner.width) / 2)
        .clamp(0, i32::from(snapshot.map.width.saturating_sub(inner.width)));
    let origin_y = (snapshot.player.position.y - i32::from(inner.height) / 2).clamp(
        0,
        i32::from(snapshot.map.height.saturating_sub(inner.height)),
    );
    for y in 0..inner.height.min(snapshot.map.height) {
        for x in 0..inner.width.min(snapshot.map.width) {
            let map_x = (origin_x as usize) + usize::from(x);
            let map_y = (origin_y as usize) + usize::from(y);
            let tile = snapshot.map.tiles[map_y * usize::from(snapshot.map.width) + map_x];
            let (glyph, color) = match tile {
                Tile::Floor => (".", Color::Rgb(52, 66, 84)),
                Tile::Wall => ("#", MUTED),
                Tile::StairsDown => (">", Color::Yellow),
            };
            frame.buffer_mut()[(inner.x + x, inner.y + y)]
                .set_symbol(glyph)
                .set_fg(color)
                .set_bg(BACKGROUND);
        }
    }
    for entity in &snapshot.entities {
        let x = entity.position.x - origin_x;
        let y = entity.position.y - origin_y;
        if x < 0 || y < 0 || x >= i32::from(inner.width) || y >= i32::from(inner.height) {
            continue;
        }
        let player = entity.id == snapshot.player.id;
        frame.buffer_mut()[(inner.x + x as u16, inner.y + y as u16)]
            .set_symbol(&entity.glyph.to_string())
            .set_fg(if player { ACCENT } else { Color::LightRed })
            .set_style(Style::default().add_modifier(Modifier::BOLD));
    }

    let visible = usize::from(log.height.saturating_sub(2));
    let lines: Vec<_> = messages
        .iter()
        .skip(messages.len().saturating_sub(visible))
        .map(|message| Line::from(message.as_str()))
        .collect();
    frame.render_widget(Paragraph::new(lines).block(panel(" Combat log ")), log);
    frame.render_widget(
        Paragraph::new(
            "Arrows / hjkl: move & attack | . / Space: wait | r: restart | q / Esc: quit",
        )
        .style(Style::default().fg(MUTED))
        .wrap(Wrap { trim: true }),
        controls,
    );
}

fn panel(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(MUTED))
}

#[cfg(test)]
mod tests {
    use super::*;
    use project_rogue_engine_core::{Command, Engine};
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn renders_entities_health_log_and_small_terminal_without_panicking() {
        let mut engine = Engine::default();
        let mut snapshot = engine.snapshot();
        snapshot.map.tiles[1] = Tile::Wall;
        snapshot.map.tiles[2] = Tile::StairsDown;
        let messages = VecDeque::from(["Rat takes 1 damage.".to_owned()]);
        for (width, height) in [(80, 32), (30, 12), (8, 4)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| draw(frame, &snapshot, &messages))
                .unwrap();
            if width == 80 {
                let buffer = terminal.backend().buffer();
                assert_eq!(buffer[(1, 4)].symbol(), "@");
                assert_eq!(buffer[(2, 4)].symbol(), "#");
                assert_eq!(buffer[(3, 4)].symbol(), ">");
                assert_eq!(buffer[(39, 22)].symbol(), "r");
                let text: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
                assert!(text.contains("HP 10/10"));
                assert!(text.contains("Rat takes 1 damage."));
            }
        }
        for _ in 0..80 {
            engine.dispatch(Command::Wait);
        }
        let mut terminal = Terminal::new(TestBackend::new(80, 32)).unwrap();
        terminal
            .draw(|frame| draw(frame, &engine.snapshot(), &messages))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.contains("DEFEAT"));
    }
}
