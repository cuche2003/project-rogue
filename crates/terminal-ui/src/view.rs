use std::collections::VecDeque;

use project_rogue_engine_core::{GamePhase, RenderSnapshot, Tile};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

const BACKGROUND: Color = Color::Rgb(16, 20, 29);
const TEXT: Color = Color::Rgb(214, 222, 235);
const MUTED: Color = Color::Rgb(112, 131, 153);
const ACCENT: Color = Color::Rgb(104, 214, 196);

pub fn draw(
    frame: &mut Frame,
    snapshot: &RenderSnapshot,
    messages: &VecDeque<String>,
    replay: Option<(usize, usize, bool)>,
) {
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
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(if area.height >= 18 { 5 } else { 3 }),
        Constraint::Length(4),
    ])
    .areas(area);
    let hp = snapshot.player.health;
    let inventory = if snapshot.inventory.is_empty() {
        "Inventory: empty".to_owned()
    } else {
        snapshot
            .inventory
            .iter()
            .enumerate()
            .map(|(index, item)| format!("{}:{}{}", index + 1, item.glyph, item.quantity))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let phase = match snapshot.phase {
        GamePhase::Defeat => "DEFEAT - r: restart",
        GamePhase::Victory => "VICTORY - r: restart",
        GamePhase::Playing => "Explore: find > then descend",
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    format!("HP {}/{}", hp.current, hp.maximum),
                    Style::default()
                        .fg(if hp.current <= 3 {
                            Color::LightRed
                        } else {
                            ACCENT
                        })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(" | Turn {}", snapshot.turn)),
            ]),
            Line::from(format!(
                "{} | F{}/{} | {}",
                phase, snapshot.floor, snapshot.floor_count, inventory
            )),
        ]),
        status,
    );

    let border = panel(" Dungeon | @ you > stairs ");
    let inner = border.inner(map_area);
    frame.render_widget(border, map_area);
    // Follow player when terminal cannot fit full map.
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
            let index = map_y * usize::from(snapshot.map.width) + map_x;
            let Some(tile) = snapshot.map.tiles[index] else {
                continue;
            };
            let glyph = match tile {
                Tile::Floor => ".",
                Tile::Wall => "#",
                Tile::StairsDown => ">",
            };
            let style = if snapshot.map.visible[index] {
                Style::default().fg(match tile {
                    Tile::Floor => MUTED,
                    Tile::Wall => TEXT,
                    Tile::StairsDown => Color::Yellow,
                })
            } else {
                Style::default()
                    .fg(Color::Rgb(65, 76, 92))
                    .add_modifier(Modifier::DIM)
            };
            frame.buffer_mut()[(inner.x + x, inner.y + y)]
                .set_symbol(glyph)
                .set_style(style);
        }
    }
    for item in &snapshot.items {
        let Some(position) = item.position else {
            continue;
        };
        let x = position.x - origin_x;
        let y = position.y - origin_y;
        if x < 0 || y < 0 || x >= i32::from(inner.width) || y >= i32::from(inner.height) {
            continue;
        }
        frame.buffer_mut()[(inner.x + x as u16, inner.y + y as u16)]
            .set_symbol(&item.glyph.to_string())
            .set_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
    }
    // Core already filters hidden entities; render supplied entities only.
    for entity in &snapshot.entities {
        let x = entity.position.x - origin_x;
        let y = entity.position.y - origin_y;
        if x < 0 || y < 0 || x >= i32::from(inner.width) || y >= i32::from(inner.height) {
            continue;
        }
        let player = entity.id == snapshot.player.id;
        frame.buffer_mut()[(inner.x + x as u16, inner.y + y as u16)]
            .set_symbol(&entity.glyph.to_string())
            .set_style(
                Style::default()
                    .fg(if player { ACCENT } else { Color::LightRed })
                    .add_modifier(Modifier::BOLD)
                    .remove_modifier(Modifier::DIM),
            );
    }

    let visible = usize::from(log.height.saturating_sub(2));
    let lines: Vec<_> = messages
        .iter()
        .skip(messages.len().saturating_sub(visible))
        .map(|message| Line::from(message.as_str()))
        .collect();
    frame.render_widget(Paragraph::new(lines).block(panel(" Log ")), log);
    frame.render_widget(
        Paragraph::new(match replay {
            Some((index, total, playing)) => format!(
                "Replay {index}/{total} {}\nSpace/Enter: step p: play/pause\nq/Esc: quit",
                if index == total {
                    "[complete]"
                } else if playing {
                    "[playing]"
                } else {
                    "[paused]"
                }
            ),
            None => "Arrows/hjkl: move/attack\n./Space: wait  >: descend\nr: restart  q/Esc: quit"
                .to_owned(),
        })
        .style(Style::default().fg(MUTED)),
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
    use project_rogue_engine_core::{
        EntityId, EntitySnapshot, HealthSnapshot, MapSnapshot, PlayerSnapshot, Position,
    };
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    fn snapshot() -> RenderSnapshot {
        let id = EntityId(5);
        let position = Position { x: 1, y: 1 };
        RenderSnapshot {
            phase: GamePhase::Playing,
            turn: 7,
            floor: 1,
            floor_count: 3,
            seed: 0,
            map: MapSnapshot {
                width: 8,
                height: 4,
                tiles: vec![Some(Tile::Floor); 32],
                visible: vec![true; 32],
            },
            player: PlayerSnapshot {
                id,
                position,
                health: HealthSnapshot {
                    current: 10,
                    maximum: 10,
                },
            },
            entities: vec![EntitySnapshot {
                id,
                position,
                glyph: '@',
                name: "Player".to_owned(),
            }],
            items: vec![],
            inventory: vec![],
            messages: vec![],
        }
    }

    fn render(snapshot: &RenderSnapshot, width: u16, height: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| {
                draw(
                    frame,
                    snapshot,
                    &VecDeque::from(["Latest message.".to_owned()]),
                    None,
                );
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn text(buffer: &Buffer) -> String {
        buffer.content().iter().map(|cell| cell.symbol()).collect()
    }

    #[test]
    fn renders_unknown_blank_and_remembered_terrain_dim() {
        let mut snapshot = snapshot();
        snapshot.map.tiles[0] = None;
        for (x, tile) in [Tile::Floor, Tile::Wall, Tile::StairsDown]
            .into_iter()
            .enumerate()
        {
            let index = 1 + x * 2;
            snapshot.map.tiles[index] = Some(tile);
            snapshot.map.tiles[index + 1] = Some(tile);
            snapshot.map.visible[index] = false;
        }
        let buffer = render(&snapshot, 80, 24);
        assert_eq!(buffer[(1, 3)].symbol(), " ");
        for (x, glyph) in [(2, "."), (4, "#"), (6, ">")] {
            let remembered = &buffer[(x, 3)];
            let visible = &buffer[(x + 1, 3)];
            assert_eq!(remembered.symbol(), glyph);
            assert_eq!(visible.symbol(), glyph);
            assert!(remembered.modifier.contains(Modifier::DIM));
            assert!(!visible.modifier.contains(Modifier::DIM));
            assert_ne!(remembered.fg, visible.fg);
        }
        assert_eq!(buffer[(2, 4)].symbol(), "@");
    }

    #[test]
    fn renders_only_supplied_entities_without_inferring_victory() {
        let mut snapshot = snapshot();
        let buffer = render(&snapshot, 80, 24);
        assert_eq!(buffer[(7, 5)].symbol(), ".");
        assert!(text(&buffer).contains("Explore: find > then descend"));
        assert!(!text(&buffer).contains("defeated"));
        snapshot.entities.push(EntitySnapshot {
            id: EntityId(42),
            position: Position { x: 6, y: 2 },
            glyph: 'r',
            name: "Rat".to_owned(),
        });
        assert_eq!(render(&snapshot, 80, 24)[(7, 5)].symbol(), "r");
    }

    #[test]
    fn replay_controls_show_paused_playing_and_complete_including_empty() {
        for (index, total, playing, label) in [
            (0, 2, false, "[paused]"),
            (1, 2, true, "[playing]"),
            (2, 2, false, "[complete]"),
            (0, 0, false, "[complete]"),
        ] {
            let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
            terminal
                .draw(|frame| {
                    draw(
                        frame,
                        &snapshot(),
                        &VecDeque::new(),
                        Some((index, total, playing)),
                    )
                })
                .unwrap();
            assert!(text(terminal.backend().buffer()).contains(label));
        }
    }

    #[test]
    fn renders_phase_from_snapshot() {
        let mut snapshot = snapshot();
        for (phase, label) in [
            (GamePhase::Victory, "VICTORY - r: restart"),
            (GamePhase::Defeat, "DEFEAT - r: restart"),
        ] {
            snapshot.phase = phase;
            assert!(text(&render(&snapshot, 30, 12)).contains(label));
        }
    }

    #[test]
    fn compact_terminal_keeps_status_log_and_all_controls_visible() {
        let snapshot = snapshot();
        let rendered = text(&render(&snapshot, 30, 12));
        for label in [
            "HP 10/10 | Turn 7",
            "Explore: find > then descend",
            "Latest message.",
            "Arrows/hjkl: move/attack",
            "./Space: wait  >: descend",
            "r: restart  q/Esc: quit",
        ] {
            assert!(rendered.contains(label), "missing: {label}");
        }
        assert!(text(&render(&snapshot, 8, 4)).contains("Terminal"));
    }
}
