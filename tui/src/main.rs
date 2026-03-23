mod ui;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
    MouseEventKind,
};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use futures::StreamExt;
use p2pchat_types::api::{UiClientRequest, WriteEvent};
use p2pchat_types::{Contact, Message};
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::ListState;
use ratatui::Terminal;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Contacts,
    Friends,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    ContactList,
    Chat,
    FriendsSearch,
    FriendsPending,
    FriendsIncoming,
    FriendsSearchResults,
    FriendsMdnsResults,
}

pub struct App {
    pub selected_tab: Tab,
    pub contacts: Vec<Contact>,
    pub contact_list_state: ListState,
    pub messages: Vec<Message>,
    pub input: String,
    pub input_mode: InputMode,
    pub focus: Focus,
    pub username: String,
    pub relay_connected: bool,
    pub relay_addr: String,
    pub request_tx: mpsc::UnboundedSender<UiClientRequest>,
    pub should_quit: bool,
    // Friends tab state
    pub friends_search_input: String,
    pub pending_requests: Vec<Contact>,
    pub pending_list_state: ListState,
    pub incoming_requests: Vec<Contact>,
    pub incoming_list_state: ListState,
    pub search_results: Vec<Contact>,
    pub search_results_list_state: ListState,
    pub mdns_results: Vec<Contact>,
    pub mdns_results_list_state: ListState,
    /// Stored button areas for incoming request accept/deny hit testing
    /// Each entry: (accept_rect, deny_rect)
    pub incoming_button_areas: Vec<(ratatui::layout::Rect, ratatui::layout::Rect)>,
}

impl App {
    fn new(request_tx: mpsc::UnboundedSender<UiClientRequest>) -> Self {
        Self {
            selected_tab: Tab::Contacts,
            contacts: Vec::new(),
            contact_list_state: ListState::default(),
            messages: Vec::new(),
            input: String::new(),
            input_mode: InputMode::Normal,
            focus: Focus::ContactList,
            username: String::new(),
            relay_connected: false,
            relay_addr: String::new(),
            request_tx,
            should_quit: false,
            friends_search_input: String::new(),
            pending_requests: Vec::new(),
            pending_list_state: ListState::default(),
            incoming_requests: Vec::new(),
            incoming_list_state: ListState::default(),
            search_results: Vec::new(),
            search_results_list_state: ListState::default(),
            mdns_results: Vec::new(),
            mdns_results_list_state: ListState::default(),
            incoming_button_areas: Vec::new(),
        }
    }

    pub fn selected_contact(&self) -> Option<&Contact> {
        self.contact_list_state
            .selected()
            .and_then(|i| self.contacts.get(i))
    }

    pub fn selected_contact_name(&self) -> &str {
        self.selected_contact()
            .map(|c| c.name.as_str())
            .unwrap_or("nobody")
    }

    fn next_contact(&mut self) {
        if self.contacts.is_empty() {
            return;
        }
        let i = match self.contact_list_state.selected() {
            Some(i) => (i + 1).min(self.contacts.len() - 1),
            None => 0,
        };
        self.contact_list_state.select(Some(i));
    }

    fn previous_contact(&mut self) {
        if self.contacts.is_empty() {
            return;
        }
        let i = match self.contact_list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.contact_list_state.select(Some(i));
    }

    fn next_tab(&mut self) {
        self.selected_tab = match self.selected_tab {
            Tab::Contacts => Tab::Friends,
            Tab::Friends => Tab::Contacts,
        };
    }

    fn focused_list_next(&mut self) {
        match self.focus {
            Focus::ContactList => self.next_contact(),
            Focus::FriendsPending => list_next(&mut self.pending_list_state, self.pending_requests.len()),
            Focus::FriendsIncoming => list_next(&mut self.incoming_list_state, self.incoming_requests.len()),
            Focus::FriendsSearchResults => list_next(&mut self.search_results_list_state, self.search_results.len()),
            Focus::FriendsMdnsResults => list_next(&mut self.mdns_results_list_state, self.mdns_results.len()),
            _ => {}
        }
    }

    fn focused_list_prev(&mut self) {
        match self.focus {
            Focus::ContactList => self.previous_contact(),
            Focus::FriendsPending => list_prev(&mut self.pending_list_state),
            Focus::FriendsIncoming => list_prev(&mut self.incoming_list_state),
            Focus::FriendsSearchResults => list_prev(&mut self.search_results_list_state),
            Focus::FriendsMdnsResults => list_prev(&mut self.mdns_results_list_state),
            _ => {}
        }
    }

    fn accept_selected_incoming(&self) {
        if let Some(i) = self.incoming_list_state.selected() {
            if let Some(contact) = self.incoming_requests.get(i) {
                let _ = self.request_tx.send(UiClientRequest {
                    req_id: uuid::Uuid::new_v4(),
                    event: p2pchat_types::api::UiClientEvent::EventRequiringDial(
                        p2pchat_types::api::UiClientEventRequiringDial {
                            peer_id: contact.peer_id.clone(),
                            event: p2pchat_types::api::UiClientEventRequiringDialMessage::AcceptFriendRequest {
                                peer_id: contact.peer_id.clone(),
                            },
                        },
                    ),
                });
            }
        }
    }

    fn deny_selected_incoming(&self) {
        if let Some(i) = self.incoming_list_state.selected() {
            if let Some(contact) = self.incoming_requests.get(i) {
                let _ = self.request_tx.send(UiClientRequest {
                    req_id: uuid::Uuid::new_v4(),
                    event: p2pchat_types::api::UiClientEvent::EventRequiringDial(
                        p2pchat_types::api::UiClientEventRequiringDial {
                            peer_id: contact.peer_id.clone(),
                            event: p2pchat_types::api::UiClientEventRequiringDialMessage::DenyFriendRequest {
                                peer_id: contact.peer_id.clone(),
                            },
                        },
                    ),
                });
            }
        }
    }
}

fn list_next(state: &mut ListState, len: usize) {
    if len == 0 {
        return;
    }
    let i = match state.selected() {
        Some(i) => (i + 1).min(len - 1),
        None => 0,
    };
    state.select(Some(i));
}

fn list_prev(state: &mut ListState) {
    let i = match state.selected() {
        Some(i) => i.saturating_sub(1),
        None => 0,
    };
    state.select(Some(i));
}

async fn read_write_event(
    sock_read: &mut (impl AsyncReadExt + Unpin),
) -> anyhow::Result<WriteEvent> {
    let len = sock_read.read_u64().await?;
    let mut buf = vec![0u8; len as usize];
    sock_read.read_exact(&mut buf).await?;
    let event: WriteEvent = postcard::from_bytes(&buf)?;
    Ok(event)
}

async fn send_request(
    sock_write: &mut (impl AsyncWriteExt + Unpin),
    request: &UiClientRequest,
) -> anyhow::Result<()> {
    let serialized = postcard::to_allocvec(request)?;
    sock_write.write_u64(serialized.len() as u64).await?;
    sock_write.write_all(&serialized).await?;
    Ok(())
}

fn handle_write_event(app: &mut App, event: WriteEvent) {
    match event {
        WriteEvent::ReceiveMessage(msg) => {
            app.messages.push(msg);
        }
        WriteEvent::DiscoverMdnsContact { peer_id, name } => {
            let contact = Contact {
                peer_id,
                name: name.unwrap_or_default(),
                discovery_type: p2pchat_types::DiscoveryType::Mdns,
            };
            if !app.contacts.iter().any(|c| c.peer_id == contact.peer_id) {
                app.contacts.push(contact);
            }
            if app.contact_list_state.selected().is_none() && !app.contacts.is_empty() {
                app.contact_list_state.select(Some(0));
            }
        }
        WriteEvent::MdnsPeerDisconnected { peer_id } => {
            app.contacts.retain(|c| c.peer_id != peer_id);
            if let Some(sel) = app.contact_list_state.selected() {
                if sel >= app.contacts.len() {
                    app.contact_list_state
                        .select(if app.contacts.is_empty() {
                            None
                        } else {
                            Some(app.contacts.len() - 1)
                        });
                }
            }
        }
        WriteEvent::MdnsNameResolved { peer_id, name } => {
            if let Some(c) = app.contacts.iter_mut().find(|c| c.peer_id == peer_id) {
                c.name = name;
            }
        }
        WriteEvent::RelayServerConnection(relay_event) => match relay_event.0 {
            Ok(success) => {
                app.relay_connected = true;
                app.relay_addr = success.relay_addr;
            }
            Err(_) => {
                app.relay_connected = false;
            }
        },
        WriteEvent::EventResponse(_response) => {
            // TODO: handle event responses
        }
        WriteEvent::CriticalFailure(_) => {
            app.should_quit = true;
        }
        WriteEvent::ReceiveFriendRequest => {
            // TODO: handle friend request notification
        }
        WriteEvent::DcutrConnection(_) => {
            // TODO: handle dcutr connection events
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let stream = UnixStream::connect("/tmp/p2p-chat.sock").await?;
    let (mut sock_read, mut sock_write) = stream.into_split();

    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<UiClientRequest>();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(request_tx);
    let mut event_stream = event::EventStream::new();

    let result = loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if app.should_quit {
            break Ok(());
        }

        tokio::select! {
            Some(Ok(ev)) = event_stream.next() => {
                match ev {
                    Event::Key(key) => handle_key_event(&mut app, key),
                    Event::Mouse(mouse) => handle_mouse_event(&mut app, mouse),
                    _ => {}
                }
            }
            result = read_write_event(&mut sock_read) => {
                match result {
                    Ok(event) => handle_write_event(&mut app, event),
                    Err(_) => break Ok(()),
                }
            }
            Some(request) = request_rx.recv() => {
                send_request(&mut sock_write, &request).await?;
            }
        }
    };

    disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn handle_key_event(app: &mut App, key: event::KeyEvent) {
    match app.input_mode {
        InputMode::Normal => match key.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.should_quit = true;
            }
            KeyCode::Tab => app.next_tab(),
            KeyCode::Char('j') | KeyCode::Down => app.focused_list_next(),
            KeyCode::Char('k') | KeyCode::Up => app.focused_list_prev(),
            KeyCode::Char('l') | KeyCode::Right => {
                app.focus = match app.selected_tab {
                    Tab::Contacts => Focus::Chat,
                    Tab::Friends => next_friends_focus(app.focus),
                };
            }
            KeyCode::Char('h') | KeyCode::Left => {
                app.focus = match app.selected_tab {
                    Tab::Contacts => Focus::ContactList,
                    Tab::Friends => prev_friends_focus(app.focus),
                };
            }
            KeyCode::Char('i') | KeyCode::Enter => match app.focus {
                Focus::Chat => app.input_mode = InputMode::Editing,
                Focus::FriendsSearch => app.input_mode = InputMode::Editing,
                Focus::FriendsIncoming => {
                    // Enter on incoming = accept
                    app.accept_selected_incoming();
                }
                _ => {}
            },
            KeyCode::Char('x') | KeyCode::Delete => {
                if app.focus == Focus::FriendsIncoming {
                    app.deny_selected_incoming();
                }
            }
            KeyCode::Char('1') if app.selected_tab == Tab::Friends => {
                app.focus = Focus::FriendsPending;
            }
            KeyCode::Char('2') if app.selected_tab == Tab::Friends => {
                app.focus = Focus::FriendsIncoming;
            }
            KeyCode::Char('3') if app.selected_tab == Tab::Friends => {
                app.focus = Focus::FriendsSearchResults;
            }
            KeyCode::Char('4') if app.selected_tab == Tab::Friends => {
                app.focus = Focus::FriendsMdnsResults;
            }
            _ => {}
        },
        InputMode::Editing => match key.code {
            KeyCode::Esc => app.input_mode = InputMode::Normal,
            KeyCode::Enter => {
                if app.focus == Focus::FriendsSearch {
                    let query = app.friends_search_input.clone();
                    if !query.is_empty() {
                        let _ = app.request_tx.send(UiClientRequest {
                            req_id: uuid::Uuid::new_v4(),
                            event: p2pchat_types::api::UiClientEvent::SearchUsername {
                                username: query,
                            },
                        });
                    }
                } else {
                    // Chat input
                    // TODO: send message via request_tx
                    app.input.clear();
                }
            }
            KeyCode::Backspace => {
                if app.focus == Focus::FriendsSearch {
                    app.friends_search_input.pop();
                } else {
                    app.input.pop();
                }
            }
            KeyCode::Char(c) => {
                if app.focus == Focus::FriendsSearch {
                    app.friends_search_input.push(c);
                } else {
                    app.input.push(c);
                }
            }
            _ => {}
        },
    }
}

fn next_friends_focus(current: Focus) -> Focus {
    match current {
        Focus::ContactList => Focus::FriendsSearch,
        Focus::FriendsSearch => Focus::FriendsPending,
        Focus::FriendsPending => Focus::FriendsIncoming,
        Focus::FriendsIncoming => Focus::FriendsSearchResults,
        Focus::FriendsSearchResults => Focus::FriendsMdnsResults,
        _ => current,
    }
}

fn prev_friends_focus(current: Focus) -> Focus {
    match current {
        Focus::FriendsMdnsResults => Focus::FriendsSearchResults,
        Focus::FriendsSearchResults => Focus::FriendsIncoming,
        Focus::FriendsIncoming => Focus::FriendsPending,
        Focus::FriendsPending => Focus::FriendsSearch,
        Focus::FriendsSearch => Focus::ContactList,
        _ => Focus::ContactList,
    }
}

fn handle_mouse_event(app: &mut App, mouse: event::MouseEvent) {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let col = mouse.column;
            let row = mouse.row;

            // Left ~30 columns is the contact list area
            if col < 30 {
                app.focus = Focus::ContactList;
                let row = row as usize;
                if row >= 4 {
                    let contact_idx = row.saturating_sub(4) + app.contact_list_state.offset();
                    if contact_idx < app.contacts.len() {
                        app.contact_list_state.select(Some(contact_idx));
                    }
                }
            } else if app.selected_tab == Tab::Friends {
                // Check accept/deny button hits on incoming requests
                for (i, (accept_rect, deny_rect)) in
                    app.incoming_button_areas.iter().enumerate()
                {
                    if rect_contains(accept_rect, col, row) {
                        app.incoming_list_state.select(Some(i));
                        app.focus = Focus::FriendsIncoming;
                        app.accept_selected_incoming();
                        return;
                    }
                    if rect_contains(deny_rect, col, row) {
                        app.incoming_list_state.select(Some(i));
                        app.focus = Focus::FriendsIncoming;
                        app.deny_selected_incoming();
                        return;
                    }
                }
                // Generic right-panel click — just set focus to search input
                app.focus = Focus::FriendsSearch;
                app.input_mode = InputMode::Editing;
            } else {
                app.focus = Focus::Chat;
                app.input_mode = InputMode::Editing;
            }
        }
        MouseEventKind::ScrollDown => app.focused_list_next(),
        MouseEventKind::ScrollUp => app.focused_list_prev(),
        _ => {}
    }
}

fn rect_contains(rect: &ratatui::layout::Rect, col: u16, row: u16) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}
