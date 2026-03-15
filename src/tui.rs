pub mod types;
mod widgets;
use std::str::FromStr;

use crate::db::types::DiscoveryType;
use crate::network::Client;
use crate::tui::types::Contact;
use crossterm::event::KeyCode;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use futures::{FutureExt, StreamExt};
use libp2p::PeerId;
use num_enum::TryFromPrimitive;
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode::Char;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets::ListItem;
use ratatui::widgets::Paragraph;
use ratatui::widgets::{Block, List, ListDirection, ListState, Scrollbar, ScrollbarState};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio_rusqlite::fallible_iterator::FallibleIterator;
use tokio_rusqlite::params;
use tokio_util::sync::CancellationToken;
use types::{
    App, ContactPage, Event, FriendRequestPage, Key, Message, MoveHorizontal, MoveVertical,
    Tabline, Tui,
};

impl Tui {
    pub fn start(&mut self) {
        // let tick_delay = std::time::Duration::from_secs_f64(1.0 / self.tick_rate);
        // let render_delay = std::time::Duration::from_secs_f64(1.0 / self.frame_rate);
        let _event_tx = self.event_tx.clone();
        self.task = Some(tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            // let mut tick_interval = tokio::time::interval(tick_delay);
            // let mut render_interval = tokio::time::interval(render_delay);
            _event_tx.send(Event::Init).unwrap();
            loop {
                // let tick_delay = tick_interval.tick();
                // let render_delay = render_interval.tick();
                let crossterm_event = reader.next().fuse();
                tokio::select! {
                  maybe_event = crossterm_event => {
                    match maybe_event {
                      Some(Ok(evt)) =>
                        match evt {
                          crossterm::event::Event::Key(key) => {
                            if key.kind == KeyEventKind::Press {
                              _event_tx.send(Event::Key(key)).unwrap();
                            }
                          },
                          _ => { },
                        }

                      Some(Err(_)) => {
                        _event_tx.send(Event::Error).unwrap();
                      }
                      None => {},
                    }
                  },
                  // _ = tick_delay => {
                  //     _event_tx.send(Event::Tick).unwrap();
                  // },
                  // _ = render_delay => {
                  //     _event_tx.send(Event::Render).unwrap();
                  // },
                }
            }
        }));
    }
    // pub fn tick_rate(self, v: f64) -> Self {
    //     Self {
    //         terminal: self.terminal,
    //         task: self.task,
    //         event_rx: self.event_rx,
    //         event_tx: self.event_tx,
    //         frame_rate: self.frame_rate,
    //         tick_rate: v,
    //     }
    // }
    // pub fn frame_rate(self, v: f64) -> Self {
    //     Self {
    //         terminal: self.terminal,
    //         task: self.task,
    //         event_rx: self.event_rx,
    //         event_tx: self.event_tx,
    //         frame_rate: v,
    //         tick_rate: self.tick_rate,
    //     }
    // }
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Event>();
        Self {
            event_rx: rx,
            event_tx: tx,
            terminal: ratatui::init(),
            frame_rate: 30.0,
            tick_rate: 1.0,
            task: None,
        }
    }
    pub fn event_tx(&self) -> UnboundedSender<Event> {
        self.event_tx.clone()
    }
    pub async fn next(&mut self) -> Option<Event> {
        return self.event_rx.recv().await;
    }
    pub fn exit(self) {
        if let Some(task) = self.task {
            task.abort();
        }
        ratatui::restore();
    }
}
async fn handle_event(app: &mut App, event: Event) {
    // switch tabline -> SHIFT + H/L
    // switch between selectable widgets -> CTRL + H/J/K/L
    match event {
        Event::Key(key) => match (key.code, key.modifiers) {
            (KeyCode::Esc, KeyModifiers::NONE) => {
                app.should_quit = true;
                app.token.cancel();
                return;
            }
            (Char('H'), KeyModifiers::SHIFT) => {
                tracing::info!("changing selected tab {:?}", app.selected_tab);
                app.selected_tab = app.selected_tab.left();
                return;
            }
            (Char('L'), KeyModifiers::SHIFT) => {
                tracing::info!("changing selected tab {:?}", app.selected_tab);
                app.selected_tab = app.selected_tab.right();
                return;
            }
            (Key::LEFT | Key::RIGHT | Key::UP | Key::DOWN, KeyModifiers::CONTROL) => {
                app.selected_tab = match key.code {
                    Key::LEFT => match app.selected_tab {
                        Tabline::Chatting(c) => Tabline::Chatting(c.left()),
                        Tabline::FriendRequests(f) => Tabline::FriendRequests(f),
                    },
                    Key::RIGHT => match app.selected_tab {
                        Tabline::Chatting(c) => Tabline::Chatting(c.right()),
                        Tabline::FriendRequests(f) => Tabline::FriendRequests(f),
                    },
                    Key::UP => match app.selected_tab {
                        Tabline::Chatting(c) => Tabline::Chatting(c.up()),
                        Tabline::FriendRequests(f) => Tabline::FriendRequests(f.up()),
                    },
                    Key::DOWN => match app.selected_tab {
                        Tabline::Chatting(c) => Tabline::Chatting(c.down()),
                        Tabline::FriendRequests(f) => Tabline::FriendRequests(f.down()),
                    },
                    _ => unreachable!(),
                };
                return;
            }
            _ => {}
        },
        Event::MessageReceived(message) => {
            // TODO: actually handle
            app.chat.push(message);
            return;
        }
        Event::AddContact(contact) => {
            // TODO: actually handle
            if !app.contacts.contains(&contact) {
                app.contacts.push(contact);
            }
            return;
        }
        Event::ReloadContacts(contacts) => {
            app.contacts = contacts;
            return;
        }
        Event::EditContactName(contact) => {
            // TODO: actually handle
            if let Some(idx) = app
                .contacts
                .iter()
                .position(|x| x.peer_id == contact.peer_id)
            {
                let c = app.contacts.get_mut(idx).expect("unreachable");
                c.name = contact.name;
            }
            return;
        }
        Event::MdnsSearchRefresh => {
            let contacts: tokio_rusqlite::Result<Vec<Contact>> = app.sqlite
                .call(|c| {
                    let mut stmt = c.prepare("SELECT name, peer_id, discovery_type FROM contacts WHERE discovery_type = ?").unwrap();
                    let rows = stmt.query_map(params![DiscoveryType::Mdns as u8], |r| { Ok(Contact {
                        name: r.get(0)?,
                        peer_id: r.get(1)?,
                        discovery_type: DiscoveryType::try_from_primitive(r.get(2)?).unwrap()
                    })}).unwrap();

                    let mut contacts = Vec::new();
                    for r in rows {
                        contacts.push(r?);
                    }
                    Ok(contacts)
                })
                .await;
            app.friend_search_results = contacts.unwrap();
            return;
        }
        Event::SearchResult(c) => {
            app.friend_search_results = c;
            return;
        }
        Event::Init => {}
        _ => {}
    };
    match &app.selected_tab {
        Tabline::Chatting(contact) => match contact {
            ContactPage::ContactList => handle_contact_list(app, event).await,
            ContactPage::Chat => handle_chat(app, event).await,
            ContactPage::CallButton => handle_call_button(app, event),
        },
        Tabline::FriendRequests(fr) => match fr {
            FriendRequestPage::RequestList => handle_request_list(app, event),
            FriendRequestPage::Search => handle_search(app, event),
        },
    }
}
fn get_message_log(
    conn: &mut tokio_rusqlite::rusqlite::Connection,
    peer_id: String,
) -> tokio_rusqlite::Result<Vec<types::Message>> {
    let sql = "SELECT m.id, m.content, m.status, c.name, c.discovery_type FROM messages AS m INNER JOIN contacts AS c ON m.contact_id = c.peer_id WHERE contact_id = ?";
    let mut stmt = conn.prepare(sql).unwrap();

    let mut rows = stmt.query(params![peer_id])?;
    let mut log = Vec::new();
    while let Ok(Some(r)) = rows.next() {
        let m = types::Message {
            id: uuid::Uuid::from_str(r.get::<usize, String>(0)?.as_ref()).unwrap(),
            content: r.get(1)?,
            status: crate::db::types::MessageStatus::try_from_primitive(r.get(2)?).unwrap(),
            sender: types::Contact {
                name: r.get(3)?,
                discovery_type: DiscoveryType::try_from_primitive(r.get(4)?).unwrap(),
                peer_id: peer_id.to_string(),
            },
        };
        log.push(m);
    }
    Ok(log)
}
async fn handle_contact_list(app: &mut App, event: Event) {
    if let Event::Key(key) = event {
        match key.code {
            Key::RIGHT => app.selected_tab = Tabline::Chatting(ContactPage::Chat),
            Key::UP => {
                app.selected_contact.select_previous();
                let selected = app.get_selected_peer().unwrap();
                let messages = app
                    .sqlite
                    .call(move |c| get_message_log(c, selected.to_string()))
                    .await
                    .unwrap();
                app.chat = messages;
            }
            Key::DOWN | KeyCode::Enter => {
                app.selected_contact.select_next();
                let selected = app.get_selected_peer().unwrap();
                let messages = app
                    .sqlite
                    .call(move |c| get_message_log(c, selected.to_string()))
                    .await
                    .unwrap();
                app.chat = messages;
                // dsdsdsd
                // dsdsdsd
            }
            _ => (),
        };
        // TODO: Load the conversation into chat log
    }
}
async fn handle_chat(app: &mut App, event: Event) {
    if let Event::Key(key) = event {
        match key.code {
            KeyCode::Backspace => {
                app.buffer.pop();
            }
            KeyCode::Enter => {
                let receiver = app.get_selected_peer().unwrap();
                app.client.send_message(receiver, app.buffer.clone()).await;
                // add the message to our chat log
                app.chat.push(Message {
                    sender: Contact {
                        peer_id: app.client.id.to_string(),
                        name: "You".to_string(),
                        discovery_type: DiscoveryType::You,
                    },
                    content: app.buffer.clone(),
                    id: uuid::Uuid::new_v4(),
                    status: types::MessageStatus::SentOffNotRead,
                });
                // clear the chat input
                app.buffer.clear();
            }
            Char(ch) => app.buffer.push(ch),
            _ => unimplemented!(),
        }
    }
}
fn handle_call_button(app: &mut App, event: Event) {
    unimplemented!();
}
fn handle_request_list(app: &mut App, event: Event) {
    unimplemented!();
}
fn handle_search(app: &mut App, event: Event) {
    if let Event::Key(e) = event {
        match e.code {
            KeyCode::Enter => {
                todo!();
            }
            KeyCode::Backspace => {
                app.friend_search_buffer.pop();
            }
            Char(ch) => {
                app.friend_search_buffer.push(ch);
            }
            _ => {}
        }
    }
}
fn ui(f: &mut Frame, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(3), Constraint::Fill(1)])
        .split(f.area());
    // Tabline
    let tabline = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(layout[0].offset(ratatui::layout::Offset { x: 0, y: 1 }));

    f.render_widget(Paragraph::new("Chatting").centered(), tabline[0]);
    f.render_widget(Paragraph::new("Friend requests").centered(), tabline[1]);
    match app.selected_tab {
        Tabline::Chatting(_) => {
            let main_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Percentage(20), Constraint::Fill(1)])
                .split(layout[1]);
            let chat_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Fill(1), Constraint::Length(3)])
                .split(main_layout[1]);
            // contacts
            let contact_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Length(2), Constraint::Fill(1)])
                .split(main_layout[0]);

            let contact_list = List::new(app.contacts.iter().map(|c| c.name.clone()))
                .block(Block::bordered().title("Contacts"))
                .style(Style::new().white())
                .highlight_style(Style::new().italic())
                .highlight_symbol(">>")
                .repeat_highlight_symbol(true)
                .direction(ListDirection::TopToBottom);
            f.render_stateful_widget(contact_list, contact_layout[1], &mut app.selected_contact);

            let vertical_scroll = app.selected_contact.selected().unwrap_or(0); // from app state
            let mut scrollbar_state =
                ScrollbarState::new(contact_layout[1].y.into()).position(vertical_scroll);
            let contact_scroll_bar = Scrollbar::default()
                .orientation(ratatui::widgets::ScrollbarOrientation::VerticalLeft);

            f.render_stateful_widget(contact_scroll_bar, contact_layout[0], &mut scrollbar_state);
            let chat_input =
                Paragraph::new(format!(" {} {}", ">", app.buffer.clone())).block(Block::bordered());
            let messages = app
                .chat
                .iter()
                .map(|m| Text::raw(format!("{}: {}", m.sender.name, m.content)));
            let chat_log = List::new(messages).block(Block::bordered());
            f.render_widget(chat_log, chat_layout[0]);
            f.render_widget(chat_input, chat_layout[1]);
        }
        Tabline::FriendRequests(_) => {
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Length(3), // search input
                    Constraint::Fill(1),   // results / incoming split
                ])
                .split(layout[1]);

            // Search input at top
            let search_input =
                Paragraph::new(format!(" {} {}", ">", app.friend_search_buffer.clone()))
                    .block(Block::bordered().title("Search users"));
            f.render_widget(search_input, main_layout[0]);

            // Split bottom into incoming requests (left) and search results (right)
            let bottom_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Percentage(40), Constraint::Fill(1)])
                .split(main_layout[1]);

            // Incoming friend requests list
            let incoming_items: Vec<ListItem> = app
                .incoming_requests
                .iter()
                .map(|r| {
                    ListItem::new(Line::from(vec![
                        Span::raw(r.name.clone()),
                        Span::styled("  [A] Accept  [D] Deny", Style::new().dark_gray()),
                    ]))
                })
                .collect();

            let incoming_list = List::new(incoming_items)
                .block(Block::bordered().title("Incoming Requests"))
                .style(Style::new().white())
                .highlight_style(Style::new().italic().yellow())
                .highlight_symbol(">> ")
                .direction(ListDirection::TopToBottom);

            f.render_stateful_widget(
                incoming_list,
                bottom_layout[0],
                &mut app.selected_incoming_request,
            );

            // Search results list
            let result_items: Vec<ListItem> = app
                .friend_search_results
                .iter()
                .map(|r| {
                    ListItem::new(Line::from(vec![
                        Span::raw(r.name.clone()),
                        Span::styled("  [Enter] Send request", Style::new().dark_gray()),
                    ]))
                })
                .collect();

            let result_list = List::new(result_items)
                .block(Block::bordered().title("Search Results"))
                .style(Style::new().white())
                .highlight_style(Style::new().italic().green())
                .highlight_symbol(">> ")
                .direction(ListDirection::TopToBottom);

            f.render_stateful_widget(
                result_list,
                bottom_layout[1],
                &mut app.selected_search_result,
            );
        }
    }
    // friend list
}
// App state
pub async fn run(
    client: Client,
    token: CancellationToken,
    sqlite: tokio_rusqlite::Connection,
    mut tui: Tui,
) -> anyhow::Result<()> {
    // ratatui terminal
    tui.start();

    // application state
    let mut app = App {
        selected_tab: Tabline::default(),
        should_quit: false,
        client,
        contacts: Vec::new(),
        selected_contact: ListState::default().with_selected(Some(0)),
        chat: Vec::new(),
        buffer: String::new(),
        friend_search_buffer: String::new(),
        friend_search_results: vec![],
        incoming_requests: vec![],
        selected_incoming_request: ListState::default(),
        selected_search_result: ListState::default(),
        sqlite,
        token,
    };

    loop {
        let event = tui.next().await; // blocks until next event
        let Some(event) = event else {
            continue;
        };
        // application update
        handle_event(&mut app, event).await;

        tui.terminal.draw(|f| {
            ui(f, &mut app);
        })?;

        // application exit
        if app.should_quit {
            break;
        }
    }
    tui.exit();

    Ok(())
}
