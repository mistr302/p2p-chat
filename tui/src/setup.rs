mod tracker;

use std::collections::HashMap;
use std::io;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use p2pchat_types::settings::{
    SettingInput, SettingName, SettingValue, Settings, create_project_dirs, setting_definitions,
};
use p2pchat_types::HTTP_TRACKER;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

#[derive(Debug, Clone, Copy, PartialEq)]
enum SettingType {
    HumanInput,
    Generated,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Focus {
    Setting(usize),
    SaveButton,
    CancelButton,
}

struct SettingField {
    name: SettingName,
    label: &'static str,
    value: String,
    setting_type: SettingType,
    generator: Option<fn() -> SettingValue>,
}

struct SetupApp {
    fields: Vec<SettingField>,
    focus: Focus,
    should_quit: bool,
    should_save: bool,
    http_tracker: String,
    reqwest_client: reqwest::Client,
    keypair: Option<p2pchat_types::Keypair>,
    username_status: Option<String>, // Shows availability or error message
}

impl SetupApp {
    fn new(http_tracker: String) -> Self {
        let existing = Settings::load().ok();

        let fields: Vec<SettingField> = setting_definitions()
            .iter()
            .map(|def| {
                let current_value = existing
                    .as_ref()
                    .and_then(|s| s.get(&def.name))
                    .and_then(|v| {
                        if let SettingValue::String(Some(s)) = v {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                let (setting_type, generator) = match def.input {
                    SettingInput::HumanInput => (SettingType::HumanInput, None),
                    SettingInput::Generated(generate_fn) => (SettingType::Generated, Some(generate_fn)),
                };

                SettingField {
                    name: def.name,
                    label: def.label,
                    value: current_value,
                    setting_type,
                    generator,
                }
            })
            .collect();

        // Load or generate keypair
        let keypair = existing
            .as_ref()
            .and_then(|s| s.get(&SettingName::KeyPair))
            .and_then(|v| {
                if let SettingValue::String(Some(key)) = v {
                    p2pchat_types::Keypair::from_protobuf_encoding(
                        &base64::Engine::decode(&base64::engine::general_purpose::STANDARD, key).ok()?,
                    )
                    .ok()
                } else {
                    None
                }
            });

        let focus = if fields.is_empty() {
            Focus::SaveButton
        } else {
            Focus::Setting(0)
        };

        Self {
            fields,
            focus,
            should_quit: false,
            should_save: false,
            http_tracker,
            reqwest_client: reqwest::Client::new(),
            keypair,
            username_status: None,
        }
    }

    fn next_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Setting(i) => {
                if i + 1 < self.fields.len() {
                    Focus::Setting(i + 1)
                } else {
                    Focus::SaveButton
                }
            }
            Focus::SaveButton => Focus::CancelButton,
            Focus::CancelButton => {
                if self.fields.is_empty() {
                    Focus::SaveButton
                } else {
                    Focus::Setting(0)
                }
            }
        };
    }

    fn prev_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Setting(0) => Focus::CancelButton,
            Focus::Setting(i) => Focus::Setting(i - 1),
            Focus::SaveButton => {
                if self.fields.is_empty() {
                    Focus::CancelButton
                } else {
                    Focus::Setting(self.fields.len() - 1)
                }
            }
            Focus::CancelButton => Focus::SaveButton,
        };
    }

    fn handle_char(&mut self, c: char) {
        if let Focus::Setting(i) = self.focus {
            if self.fields[i].setting_type == SettingType::HumanInput {
                self.fields[i].value.push(c);
            }
        }
    }

    fn handle_backspace(&mut self) {
        if let Focus::Setting(i) = self.focus {
            if self.fields[i].setting_type == SettingType::HumanInput {
                self.fields[i].value.pop();
            }
        }
    }

    async fn handle_enter_async(&mut self) {
        match self.focus {
            Focus::Setting(i) => {
                let field = &mut self.fields[i];
                if field.setting_type == SettingType::Generated {
                    // Generate new value
                    if let Some(generator) = field.generator {
                        if let SettingValue::String(Some(val)) = generator() {
                            field.value = val.clone();
                            // Update keypair if this is the KeyPair field
                            if field.name == SettingName::KeyPair {
                                self.keypair = p2pchat_types::Keypair::from_protobuf_encoding(
                                    &base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &val).ok().unwrap_or_default(),
                                )
                                .ok();
                            }
                        }
                    }
                } else if field.name == SettingName::Name && !field.value.is_empty() {
                    // Register username when pressing Enter on Name field
                    if let Some(ref keypair) = self.keypair {
                        self.username_status = Some("Registering...".to_string());
                        match tracker::register_username(
                            &self.reqwest_client,
                            keypair,
                            self.http_tracker.clone(),
                            field.value.clone(),
                        )
                        .await
                        {
                            Ok(_) => {
                                self.username_status = Some("✓ Registered!".to_string());
                                self.next_focus();
                            }
                            Err(e) => {
                                self.username_status = Some(format!("✗ Error: {}", e));
                            }
                        }
                    } else {
                        self.username_status = Some("✗ Please generate a private key first".to_string());
                    }
                } else {
                    self.next_focus();
                }
            }
            Focus::SaveButton => {
                self.should_save = true;
                self.should_quit = true;
            }
            Focus::CancelButton => {
                self.should_quit = true;
            }
        }
    }

    async fn check_username_async(&mut self) {
        if let Focus::Setting(i) = self.focus {
            let field = &self.fields[i];
            if field.name == SettingName::Name && !field.value.is_empty() {
                self.username_status = Some("Checking...".to_string());
                match tracker::check_username_availability(
                    &self.reqwest_client,
                    field.value.clone(),
                    self.http_tracker.clone(),
                )
                .await
                {
                    Ok(true) => {
                        self.username_status = Some("✓ Available".to_string());
                    }
                    Ok(false) => {
                        self.username_status = Some("✗ Not available".to_string());
                    }
                    Err(e) => {
                        self.username_status = Some(format!("✗ {}", e));
                    }
                }
            } else if field.name == SettingName::Name && field.value.is_empty() {
                self.username_status = None;
            }
        }
    }

    fn save_settings(&self) {
        let _ = create_project_dirs();

        let mut settings: HashMap<SettingName, SettingValue> = HashMap::new();

        for field in &self.fields {
            let setting_value = if field.value.is_empty() {
                SettingValue::String(None)
            } else {
                SettingValue::String(Some(field.value.clone()))
            };
            settings.insert(field.name, setting_value);
        }

        Settings::save(&settings);
    }
}

fn draw(f: &mut ratatui::Frame, app: &SetupApp) {
    let area = f.area();

    // Center the form
    let vertical_center = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Min(10),
            Constraint::Percentage(15),
        ])
        .split(area);

    let horizontal_center = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(70),
            Constraint::Percentage(15),
        ])
        .split(vertical_center[1]);

    let form_area = horizontal_center[1];

    // Draw outer block
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title(" P2P Chat Setup ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let inner = outer_block.inner(form_area);
    f.render_widget(outer_block, form_area);

    // Calculate layout: fields + button row
    let field_count = app.fields.len();
    let mut constraints: Vec<Constraint> = Vec::new();

    for _ in 0..field_count {
        constraints.push(Constraint::Length(3));
    }
    // Spacer
    constraints.push(Constraint::Min(1));
    // Button row
    constraints.push(Constraint::Length(3));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(constraints)
        .split(inner);

    // Draw fields
    for (i, field) in app.fields.iter().enumerate() {
        let is_focused = app.focus == Focus::Setting(i);
        match field.setting_type {
            SettingType::HumanInput => {
                let status = if field.name == SettingName::Name {
                    app.username_status.as_deref()
                } else {
                    None
                };
                draw_input_field(f, chunks[i], field.label, &field.value, is_focused, status);
                if is_focused {
                    let cursor_x = chunks[i].x + field.value.len() as u16 + 1;
                    let cursor_y = chunks[i].y + 1;
                    f.set_cursor_position((cursor_x, cursor_y));
                }
            }
            SettingType::Generated => {
                draw_generated_field(f, chunks[i], field.label, &field.value, is_focused);
            }
        }
    }

    // Draw buttons
    let button_area = chunks[chunks.len() - 1];
    draw_buttons(f, button_area, app.focus);
}

fn draw_input_field(
    f: &mut ratatui::Frame,
    area: Rect,
    label: &str,
    value: &str,
    focused: bool,
    status: Option<&str>,
) {
    let border_style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    let mut title = format!(" {} ", label);
    if let Some(status_text) = status {
        title.push_str(&format!(" - {}", status_text));
    }

    let display_text = if value.is_empty() && !focused {
        Span::styled("Enter value...", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(value, Style::default().fg(Color::White))
    };

    let input = Paragraph::new(Line::from(display_text)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    );

    f.render_widget(input, area);
}

fn draw_generated_field(f: &mut ratatui::Frame, area: Rect, label: &str, value: &str, focused: bool) {
    let border_style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    // Split area: value display on left, button on right
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(18)])
        .split(area);

    // Value display (truncated)
    let display_value = if value.is_empty() {
        Span::styled("Not generated", Style::default().fg(Color::DarkGray))
    } else {
        let truncated = if value.len() > 20 {
            format!("{}...", &value[..20])
        } else {
            value.to_string()
        };
        Span::styled(truncated, Style::default().fg(Color::Green))
    };

    let value_block = Paragraph::new(Line::from(display_value)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" {} ", label)),
    );
    f.render_widget(value_block, chunks[0]);

    // Generate button
    let button_style = if focused {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Magenta)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Magenta)
    };

    let button_border = if focused {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(Color::White)
    };

    let button_text = if value.is_empty() { " Generate " } else { " Regenerate " };
    let generate_button = Paragraph::new(Line::from(Span::styled(button_text, button_style)))
        .block(Block::default().borders(Borders::ALL).border_style(button_border));
    f.render_widget(generate_button, chunks[1]);
}

fn draw_buttons(f: &mut ratatui::Frame, area: Rect, focus: Focus) {
    let button_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Length(2),
            Constraint::Percentage(20),
            Constraint::Percentage(30),
        ])
        .split(area);

    let save_focused = focus == Focus::SaveButton;
    let cancel_focused = focus == Focus::CancelButton;

    // Save button
    let save_style = if save_focused {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    let save_border = if save_focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::White)
    };

    let save_button = Paragraph::new(Line::from(Span::styled("  Save  ", save_style)))
        .block(Block::default().borders(Borders::ALL).border_style(save_border));
    f.render_widget(save_button, button_chunks[1]);

    // Cancel button
    let cancel_style = if cancel_focused {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Red)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };

    let cancel_border = if cancel_focused {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::White)
    };

    let cancel_button = Paragraph::new(Line::from(Span::styled(" Cancel ", cancel_style)))
        .block(Block::default().borders(Borders::ALL).border_style(cancel_border));
    f.render_widget(cancel_button, button_chunks[3]);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse arguments
    let mut http_tracker = HTTP_TRACKER.to_string();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "-t" {
            if let Some(tracker) = args.next() {
                http_tracker = tracker;
            }
        }
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = SetupApp::new(http_tracker);

    loop {
        terminal.draw(|f| draw(f, &app))?;

        if app.should_quit {
            break;
        }

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.should_quit = true;
                }
                KeyCode::Esc => {
                    app.should_quit = true;
                }
                KeyCode::Tab | KeyCode::Down => app.next_focus(),
                KeyCode::BackTab | KeyCode::Up => app.prev_focus(),
                KeyCode::Enter => app.handle_enter_async().await,
                KeyCode::Backspace => app.handle_backspace(),
                KeyCode::Char(c) => {
                    app.handle_char(c);
                    app.check_username_async().await;
                },
                _ => {}
            }
        }
    }

    if app.should_save {
        app.save_settings();
        println!("Settings saved successfully!");
    } else {
        println!("Setup cancelled.");
    }

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
