use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, List, ListItem, ListState, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Tabs, Wrap,
};

use crate::{App, ConnectionType, Focus, InputMode, Tab};

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
            let dot_color = match app
                .connection_status
                .get(&c.peer_id)
                .copied()
                .unwrap_or(ConnectionType::NotDialed)
            {
                ConnectionType::NotDialed => Color::Red,
                ConnectionType::Dcutr => Color::Green,
                ConnectionType::Mdns => Color::Blue,
                ConnectionType::Relayed => Color::Rgb(255, 165, 0),
            };
            ListItem::new(Line::from(vec![
                Span::styled("● ", Style::default().fg(dot_color)),
                Span::raw(name),
            ]))
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
    let mut scrollbar_state = ScrollbarState::new(app.contacts.len().saturating_sub(1))
        .position(app.contact_list_state.selected().unwrap_or(0));
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
}

fn draw_right_panel(f: &mut Frame, app: &mut App, area: Rect) {
    match app.selected_tab {
        Tab::Contacts => draw_chat_panel(f, app, area),
        Tab::Friends => draw_friends_panel(f, app, area),
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

fn draw_friends_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title("Friends")
        .padding(Padding::uniform(1));

    let inner = outer_block.inner(area);
    f.render_widget(outer_block, area);

    // Vertical split: search bar on top, then the 3-column section below
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(inner);

    let search_area = vert[0];
    let lists_area = vert[1];

    // --- Search input (centered) ---
    let search_style = if app.focus == Focus::FriendsSearch {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    // Center the search box: leave 20% margin on each side
    let search_center = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(search_area);

    let display_text =
        if app.friends_search_input.is_empty() && app.input_mode != InputMode::Editing {
            Span::styled("Search users...", Style::default().fg(Color::DarkGray))
        } else {
            Span::styled(&app.friends_search_input, search_style)
        };

    let search_widget = Paragraph::new(Line::from(display_text)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(search_style)
            .title("Search"),
    );
    f.render_widget(search_widget, search_center[1]);

    if app.focus == Focus::FriendsSearch && app.input_mode == InputMode::Editing {
        let cursor_x = search_center[1].x + app.friends_search_input.len() as u16 + 1;
        let cursor_y = search_center[1].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    // --- Three columns: Pending | Incoming | (Search Results / Mdns Results) ---
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(lists_area);

    let pending_area = cols[0];
    let incoming_area = cols[1];
    let right_col_area = cols[2];

    // -- Pending list --
    draw_friends_list(
        f,
        "Pending",
        &app.pending_requests,
        &mut app.pending_list_state,
        app.focus == Focus::FriendsPending,
        pending_area,
    );

    // -- Incoming list (with accept/deny buttons) --
    draw_incoming_list(f, app, incoming_area);

    // -- Right column: split vertically into Search Results + Mdns Results --
    let right_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(right_col_area);

    draw_friends_list(
        f,
        "Search Results",
        &app.search_results,
        &mut app.search_results_list_state,
        app.focus == Focus::FriendsSearchResults,
        right_split[0],
    );

    draw_friends_list(
        f,
        "MdnsResults",
        &app.mdns_results,
        &mut app.mdns_results_list_state,
        app.focus == Focus::FriendsMdnsResults,
        right_split[1],
    );
}

/// Generic helper: renders a contact list with title, highlight, and scrollbar.
fn draw_friends_list(
    f: &mut Frame,
    title: &str,
    contacts: &[p2pchat_types::Contact],
    state: &mut ListState,
    focused: bool,
    area: Rect,
) {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    let items: Vec<ListItem> = contacts
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

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, state);

    // Scrollbar
    let scrollbar_area = area.inner(Margin {
        vertical: 1,
        horizontal: 0,
    });
    let mut scrollbar_state = ScrollbarState::new(contacts.len().saturating_sub(1))
        .position(state.selected().unwrap_or(0));
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
}

/// Renders the Incoming list with per-row accept (✓) / deny (✗) buttons.
fn draw_incoming_list(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::FriendsIncoming;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title("Incoming");

    let list_inner = block.inner(area);
    f.render_widget(block, area);

    // We render each row manually so we can place clickable buttons.
    app.incoming_button_areas.clear();

    let visible_height = list_inner.height as usize;
    let offset = app.incoming_list_state.offset();
    let selected = app.incoming_list_state.selected();

    for (vi, idx) in (offset..).take(visible_height).enumerate() {
        let Some(contact) = app.incoming_requests.get(idx) else {
            break;
        };
        let row_y = list_inner.y + vi as u16;
        let is_selected = selected == Some(idx);

        let name = if contact.name.is_empty() {
            &contact.peer_id
        } else {
            &contact.name
        };

        // Layout: "> name       ✓ ✗"
        // Reserve 6 cols on the right for " ✓ ✗" (with spacing)
        let name_width = list_inner.width.saturating_sub(6);

        let row_style = if is_selected {
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        // Prefix
        let prefix = if is_selected { "> " } else { "  " };
        let mut display_name = format!("{prefix}{name}");
        display_name.truncate(name_width as usize);

        let name_span = Span::styled(display_name, row_style);
        let name_area = Rect::new(list_inner.x, row_y, name_width, 1);
        f.render_widget(Paragraph::new(Line::from(name_span)), name_area);

        // Accept button ✓
        let accept_x = list_inner.x + name_width + 1;
        let accept_rect = Rect::new(accept_x, row_y, 1, 1);
        let accept_span = Span::styled(
            "✓",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );
        f.render_widget(Paragraph::new(Line::from(accept_span)), accept_rect);

        // Deny button ✗
        let deny_x = accept_x + 2;
        let deny_rect = Rect::new(deny_x, row_y, 1, 1);
        let deny_span = Span::styled(
            "✗",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        );
        f.render_widget(Paragraph::new(Line::from(deny_span)), deny_rect);

        app.incoming_button_areas.push((accept_rect, deny_rect));
    }

    // Scrollbar
    let scrollbar_area = area.inner(Margin {
        vertical: 1,
        horizontal: 0,
    });
    let mut scrollbar_state = ScrollbarState::new(app.incoming_requests.len().saturating_sub(1))
        .position(app.incoming_list_state.selected().unwrap_or(0));
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
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
