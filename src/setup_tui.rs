use crate::settings::{
    SettingDefinition, SettingInput, SettingName, SettingValue, Settings, create_project_dirs,
    setting_definitions,
};
use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use std::collections::HashMap;

pub fn run_setup() -> anyhow::Result<()> {
    create_project_dirs()?;
    let mut settings = Settings::load()?;
    let definitions = setting_definitions();
    let mut input_buffers = init_input_buffers(definitions, &settings);
    let mut selected_setting = ListState::default().with_selected(Some(0));
    let mut status = String::from("Ready.");
    let mut should_exit = false;
    let mut dirty = false;
    let mut show_exit_modal = false;

    let mut terminal = ratatui::init();
    while !should_exit {
        terminal.draw(|frame| {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(frame.area());
            let header = Paragraph::new(Text::from(vec![Line::from(Span::styled(
                "Arrows: move | Type: edit | Space: generate | Ctrl+S: save | Esc: exit",
                Style::default().add_modifier(Modifier::BOLD),
            ))]))
            .block(Block::default().borders(Borders::ALL).title("Settings"));
            frame.render_widget(header, layout[0]);

            let items = build_setting_items(definitions, &settings, &input_buffers);
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Configuration"),
                )
                .highlight_style(Style::new().add_modifier(Modifier::BOLD))
                .highlight_symbol(">> ");
            frame.render_stateful_widget(list, layout[1], &mut selected_setting);

            let footer = Paragraph::new(Text::from(Line::from(status.as_str())))
                .block(Block::default().borders(Borders::ALL).title("Status"))
                .wrap(Wrap { trim: true });
            frame.render_widget(footer, layout[2]);

            if show_exit_modal {
                render_exit_modal(frame, layout[1], dirty);
            }
        })?;

        match crossterm::event::read()? {
            CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                if show_exit_modal {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            save_settings(&mut settings, &input_buffers);
                            status = "Settings saved.".to_string();
                            dirty = false;
                            should_exit = true;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            should_exit = true;
                        }
                        KeyCode::Esc => {
                            show_exit_modal = false;
                        }
                        _ => {}
                    }
                    continue;
                }

                match (key.code, key.modifiers) {
                    (KeyCode::Esc, KeyModifiers::NONE) => {
                        if dirty {
                            show_exit_modal = true;
                            status = "Unsaved changes. Save before exiting? (y/n)".to_string();
                        } else {
                            should_exit = true;
                        }
                    }
                    (KeyCode::Up, _) => selected_setting.select_previous(),
                    (KeyCode::Down, _) => selected_setting.select_next(),
                    (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                        save_settings(&mut settings, &input_buffers);
                        status = "Settings saved.".to_string();
                        dirty = false;
                    }
                    (KeyCode::Backspace, _) => {
                        if let Some(idx) = selected_setting.selected() {
                            let def = &definitions[idx];
                            if matches!(def.input, SettingInput::HumanInput)
                                && let Some(buf) = input_buffers.get_mut(&def.name)
                            {
                                buf.pop();
                                apply_input_buffer(&mut settings, def, buf);
                                dirty = true;
                            }
                        }
                    }
                    (KeyCode::Char(' '), _) => {
                        if let Some(idx) = selected_setting.selected() {
                            let def = &definitions[idx];
                            if let SettingInput::Generated(generator) = def.input {
                                tracing::info!("Generating {}.", def.label);
                                settings.insert(def.name, generator());
                                dirty = true;
                                status = format!("Generating {}.", def.label);
                            } else if matches!(def.input, SettingInput::HumanInput)
                                && let Some(buf) = input_buffers.get_mut(&def.name)
                            {
                                buf.push(' ');
                                apply_input_buffer(&mut settings, def, buf);
                                dirty = true;
                            }
                        }
                    }
                    (KeyCode::Char(c), _) => {
                        if !c.is_control()
                            && let Some(idx) = selected_setting.selected()
                        {
                            let def = &definitions[idx];
                            if matches!(def.input, SettingInput::HumanInput)
                                && let Some(buf) = input_buffers.get_mut(&def.name)
                            {
                                buf.push(c);
                                apply_input_buffer(&mut settings, def, buf);
                                dirty = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    ratatui::restore();
    Ok(())
}

fn init_input_buffers(
    definitions: &[SettingDefinition],
    settings: &HashMap<SettingName, SettingValue>,
) -> HashMap<SettingName, String> {
    let mut buffers = HashMap::new();
    for def in definitions {
        if matches!(def.input, SettingInput::HumanInput) {
            let value = settings.get(&def.name);
            let buf = match value {
                Some(SettingValue::String(Some(s))) => s.clone(),
                _ => String::new(),
            };
            buffers.insert(def.name, buf);
        }
    }
    buffers
}

fn apply_input_buffer(
    settings: &mut HashMap<SettingName, SettingValue>,
    def: &SettingDefinition,
    buffer: &str,
) {
    if matches!(def.default_value, SettingValue::String(_)) {
        let value = if buffer.is_empty() {
            SettingValue::String(None)
        } else {
            SettingValue::String(Some(buffer.to_string()))
        };
        settings.insert(def.name, value);
    }
}

fn save_settings(
    settings: &mut HashMap<SettingName, SettingValue>,
    buffers: &HashMap<SettingName, String>,
) {
    for (name, buf) in buffers {
        let value = if buf.is_empty() {
            SettingValue::String(None)
        } else {
            SettingValue::String(Some(buf.clone()))
        };
        settings.insert(*name, value);
    }
    Settings::save(settings);
}

fn build_setting_items(
    definitions: &[SettingDefinition],
    settings: &HashMap<SettingName, SettingValue>,
    buffers: &HashMap<SettingName, String>,
) -> Vec<ListItem<'static>> {
    definitions
        .iter()
        .map(|def| {
            let value = match def.input {
                SettingInput::HumanInput => buffers.get(&def.name).cloned().unwrap_or_default(),
                SettingInput::Generated(_) => match settings.get(&def.name) {
                    Some(SettingValue::String(Some(value))) => {
                        if value.is_empty() {
                            "missing".to_string()
                        } else {
                            format!("{} chars", value.len())
                        }
                    }
                    Some(SettingValue::Bytes(Some(bytes))) => {
                        if bytes.is_empty() {
                            "missing".to_string()
                        } else {
                            format!("{} bytes", bytes.len())
                        }
                    }
                    Some(SettingValue::Bool(b)) => {
                        if *b {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        }
                    }
                    Some(SettingValue::Int(v)) => format!("{}", v),
                    _ => "missing".to_string(),
                },
            };

            let line = Line::from(vec![
                Span::styled(def.label, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(": "),
                Span::raw(value),
            ]);
            ListItem::new(line)
        })
        .collect()
}

fn render_exit_modal(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, dirty: bool) {
    let modal_area = centered_rect(60, 20, area);
    frame.render_widget(Clear, modal_area);
    let text = if dirty {
        "Save changes before exiting? (y/n)"
    } else {
        "Exit setup? (y/n)"
    };
    let paragraph = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            "Exit",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(text),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Confirm"))
    .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, modal_area);
}

fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
