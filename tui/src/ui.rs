use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, List, ListItem, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Tabs, Wrap,
};

use crate::{App, Focus, InputMode, Tab};

pub fn draw(f: &mut Frame, app: &mut App) {
    let outer = f.area();

    // Main layout: top content area + bottom status bar (3 rows)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(outer);

    let content_area = main_chunks[0];
    let status_area = main_chunks[1];

    // Content: left sidebar (contacts) + right panel (chat/friends)
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(1)])
        .split(content_area);

    let left_area = content_chunks[0];
    let right_area = content_chunks[1];

    draw_left_panel(f, app, left_area);
    draw_right_panel(f, app, right_area);
    draw_status_bar(f, app, status_area);
}

fn draw_left_panel(f: &mut Frame, app: &mut App, area: Rect) {
    // Split into tabs row + contact list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    // Tabs
    let tab_titles = vec![Line::from("Contacts"), Line::from("Friends")];
    let selected_tab = match app.selected_tab {
        Tab::Contacts => 0,
        Tab::Friends => 1,
    };
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title("Navigation"))
        .select(selected_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, chunks[0]);

    // Contact list
    let highlight_style = if app.focus == Focus::ContactList {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app
        .contacts
        .iter()
        .map(|c| {
            let name = if c.name.is_empty() {
                &c.peer_id
            } else {
                &c.name
            };
            ListItem::new(Line::from(Span::raw(name)))
        })
        .collect();

    let border_style = if app.focus == Focus::ContactList {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title("Contacts"),
        )
        .highlight_style(highlight_style)
        .highlight_symbol("> ");

    f.render_stateful_widget(list, chunks[1], &mut app.contact_list_state);

    // Scrollbar for contact list
    let scrollbar_area = chunks[1].inner(Margin {
        vertical: 1,
        horizontal: 0,
    });
    let mut scrollbar_state =
        ScrollbarState::new(app.contacts.len().saturating_sub(1))
            .position(app.contact_list_state.selected().unwrap_or(0));
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
}

fn draw_right_panel(f: &mut Frame, app: &mut App, area: Rect) {
    match app.selected_tab {
        Tab::Contacts => draw_chat_panel(f, app, area),
        Tab::Friends => draw_friends_panel(f, area),
    }
}

fn draw_chat_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let border_style = if app.focus == Focus::Chat {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    let chat_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(format!("Chat with @{}", app.selected_contact_name()))
        .padding(Padding::uniform(1));

    let inner = chat_block.inner(area);
    f.render_widget(chat_block, area);

    // Split inner into messages area + input area
    let chat_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(inner);

    let messages_area = chat_chunks[0];
    let input_area = chat_chunks[1];

    // Render messages with sender name grouping
    let mut lines: Vec<Line> = Vec::new();
    let mut last_sender: Option<String> = None;

    for msg in &app.messages {
        let sender_id = &msg.sender.peer_id;
        if last_sender.as_ref() != Some(sender_id) {
            if !lines.is_empty() {
                lines.push(Line::from("")); // blank line between sender groups
            }
            let display_name = if msg.sender.name.is_empty() {
                &msg.sender.peer_id
            } else {
                &msg.sender.name
            };
            lines.push(Line::from(Span::styled(
                display_name.to_string(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
            last_sender = Some(sender_id.clone());
        }
        lines.push(Line::from(Span::raw(format!("  {}", msg.content))));
    }

    // Auto-scroll: show the bottom of the messages
    let visible_height = messages_area.height as usize;
    let scroll_offset = lines.len().saturating_sub(visible_height);

    let messages_widget = Paragraph::new(Text::from(lines))
        .scroll((scroll_offset as u16, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(messages_widget, messages_area);

    // Input box
    let input_style = match app.input_mode {
        InputMode::Normal => Style::default().fg(Color::White),
        InputMode::Editing => Style::default().fg(Color::Yellow),
    };

    let placeholder = format!("Message @{}", app.selected_contact_name());
    let display_text = if app.input.is_empty() && app.input_mode == InputMode::Normal {
        Span::styled(&placeholder, Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(&app.input, input_style)
    };

    let input_widget = Paragraph::new(Line::from(display_text)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(input_style)
            .title("Input"),
    );
    f.render_widget(input_widget, input_area);

    // Show cursor in input when editing
    if app.input_mode == InputMode::Editing {
        let cursor_x = input_area.x + app.input.len() as u16 + 1; // +1 for border
        let cursor_y = input_area.y + 1; // +1 for border
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

fn draw_friends_panel(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Friends")
        .padding(Padding::uniform(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = Paragraph::new("Friends view — not yet implemented")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(text, inner);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::horizontal(2));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Build status line: [username]  [●] [relay_addr]
    let connection_dot = if app.relay_connected {
        Span::styled("●", Style::default().fg(Color::Green))
    } else {
        Span::styled("●", Style::default().fg(Color::Red))
    };

    let username_display = if app.username.is_empty() {
        "unknown"
    } else {
        &app.username
    };

    let relay_display = if app.relay_addr.is_empty() {
        "not connected"
    } else {
        &app.relay_addr
    };

    let status_line = Line::from(vec![
        Span::styled(
            username_display,
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("    "), // 4-char gap
        connection_dot,
        Span::raw(" "),
        Span::raw(relay_display),
    ]);

    let paragraph = Paragraph::new(status_line);
    f.render_widget(paragraph, inner);
}
