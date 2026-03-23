mod ui;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
    MouseEventKind,
};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use futures::StreamExt;
use p2pchat_types::api::{UiClientEvent, UiClientRequest, WriteEvent};
use p2pchat_types::{Contact, DiscoveryType, Message, MessageStatus};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::ListState;
use std::collections::HashMap;
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionType {
    NotDialed,
    Mdns,
    Dcutr,
    Relayed,
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
    // Connection tracking
    pub connection_status: HashMap<String, ConnectionType>,
    pub loaded_chat_peer: Option<String>,
    pub last_chatlog_req_id: Option<uuid::Uuid>,
    pub pending_dial_actions: HashMap<String, Vec<UiClientRequest>>,
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
            connection_status: HashMap::new(),
            loaded_chat_peer: None,
            last_chatlog_req_id: None,
            pending_dial_actions: HashMap::new(),
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
            Tab::Contacts => {
                // Switching to Friends tab - refresh friend data
                self.send_request(UiClientEvent::LoadPendingFriendRequests);
                self.send_request(UiClientEvent::LoadIncomingFriendRequests);
                Tab::Friends
            }
            Tab::Friends => Tab::Contacts,
        };
    }

    fn focused_list_next(&mut self) {
        match self.focus {
            Focus::ContactList => self.next_contact(),
            Focus::FriendsPending => {
                list_next(&mut self.pending_list_state, self.pending_requests.len())
            }
            Focus::FriendsIncoming => {
                list_next(&mut self.incoming_list_state, self.incoming_requests.len())
            }
            Focus::FriendsSearchResults => list_next(
                &mut self.search_results_list_state,
                self.search_results.len(),
            ),
            Focus::FriendsMdnsResults => {
                list_next(&mut self.mdns_results_list_state, self.mdns_results.len())
            }
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

    fn send_request(&self, event: UiClientEvent) {
        let _ = self.request_tx.send(UiClientRequest {
            req_id: uuid::Uuid::new_v4(),
            event,
        });
    }

    fn send_requiring_dial(
        &mut self,
        peer_id: &str,
        event: p2pchat_types::api::UiClientEventRequiringDialMessage,
    ) {
        let connection = self
            .connection_status
            .get(peer_id)
            .copied()
            .unwrap_or(ConnectionType::NotDialed);

        let request = UiClientRequest {
            req_id: uuid::Uuid::new_v4(),
            event: UiClientEvent::EventRequiringDial(
                p2pchat_types::api::UiClientEventRequiringDial {
                    peer_id: peer_id.to_string(),
                    event,
                },
            ),
        };

        if connection == ConnectionType::NotDialed {
            // Dial first, queue the action for when connection is established
            self.send_request(UiClientEvent::Dial {
                peer_id: peer_id.to_string(),
            });
            self.pending_dial_actions
                .entry(peer_id.to_string())
                .or_default()
                .push(request);
        } else {
            let _ = self.request_tx.send(request);
        }
    }

    fn accept_selected_incoming(&mut self) {
        if let Some(i) = self.incoming_list_state.selected() {
            if let Some(contact) = self.incoming_requests.get(i) {
                let peer_id = contact.peer_id.clone();
                self.send_requiring_dial(
                    &peer_id,
                    p2pchat_types::api::UiClientEventRequiringDialMessage::AcceptFriendRequest {
                        peer_id: peer_id.clone(),
                    },
                );
            }
        }
    }

    fn deny_selected_incoming(&mut self) {
        if let Some(i) = self.incoming_list_state.selected() {
            if let Some(contact) = self.incoming_requests.get(i) {
                let peer_id = contact.peer_id.clone();
                self.send_requiring_dial(
                    &peer_id,
                    p2pchat_types::api::UiClientEventRequiringDialMessage::DenyFriendRequest {
                        peer_id: peer_id.clone(),
                    },
                );
            }
        }
    }

    fn send_chat_message(&mut self) {
        let message_text = self.input.clone();
        if message_text.is_empty() {
            return;
        }
        let peer_id = match self.selected_contact() {
            Some(c) => c.peer_id.clone(),
            None => return,
        };

        // Optimistic local update
        let msg = Message {
            content: message_text.clone(),
            id: uuid::Uuid::new_v4(),
            sender: Contact {
                peer_id: String::new(),
                name: self.username.clone(),
                discovery_type: DiscoveryType::You,
            },
            status: MessageStatus::SentOffNotRead,
        };
        self.messages.push(msg);
        self.input.clear();

        self.send_requiring_dial(
            &peer_id,
            p2pchat_types::api::UiClientEventRequiringDialMessage::SendMessage {
                peer_id: peer_id.clone(),
                message: message_text,
            },
        );
    }

    fn fetch_chatlog_for_selected(&mut self) {
        let selected_peer_id = self
            .contact_list_state
            .selected()
            .and_then(|i| self.contacts.get(i))
            .map(|c| c.peer_id.clone());

        if selected_peer_id == self.loaded_chat_peer {
            return;
        }
        self.loaded_chat_peer = selected_peer_id.clone();
        self.messages.clear();
        if let Some(peer_id) = selected_peer_id {
            let req_id = uuid::Uuid::new_v4();
            self.last_chatlog_req_id = Some(req_id);
            let _ = self.request_tx.send(UiClientRequest {
                req_id,
                event: UiClientEvent::LoadChatlogPage {
                    from_peer_id: peer_id,
                    page: 0,
                },
            });
        }
    }

    fn flush_pending_actions(&mut self, peer_id: &str) {
        if let Some(actions) = self.pending_dial_actions.remove(peer_id) {
            for action in actions {
                let _ = self.request_tx.send(action);
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
            // Only push if the message is from the currently viewed contact
            let dominated_by_current = app
                .selected_contact()
                .is_some_and(|c| c.peer_id == msg.sender.peer_id);
            if dominated_by_current {
                app.messages.push(msg);
            }
        }
        WriteEvent::DiscoverMdnsContact { peer_id, name } => {
            app.connection_status
                .insert(peer_id.clone(), ConnectionType::Mdns);
            let contact = Contact {
                peer_id: peer_id.clone(),
                name: name.unwrap_or_default(),
                discovery_type: DiscoveryType::Mdns,
            };
            if !app.contacts.iter().any(|c| c.peer_id == contact.peer_id) {
                app.contacts.push(contact.clone());
            }
            if !app.mdns_results.iter().any(|c| c.peer_id == peer_id) {
                app.mdns_results.push(contact);
            }
            if app.contact_list_state.selected().is_none() && !app.contacts.is_empty() {
                app.contact_list_state.select(Some(0));
                app.fetch_chatlog_for_selected();
            }
            // Flush pending dial actions for this newly-connected peer
            app.flush_pending_actions(&peer_id);
        }
        WriteEvent::MdnsPeerDisconnected { peer_id } => {
            app.connection_status.remove(&peer_id);
            app.mdns_results.retain(|c| c.peer_id != peer_id);
            // Only remove purely mdns-discovered contacts (keep friends)
            app.contacts.retain(|c| {
                !(c.peer_id == peer_id && c.discovery_type == DiscoveryType::Mdns)
            });
            if let Some(sel) = app.contact_list_state.selected() {
                if sel >= app.contacts.len() {
                    app.contact_list_state.select(if app.contacts.is_empty() {
                        None
                    } else {
                        Some(app.contacts.len() - 1)
                    });
                }
            }
            app.fetch_chatlog_for_selected();
        }
        WriteEvent::MdnsNameResolved { peer_id, name } => {
            if let Some(c) = app.contacts.iter_mut().find(|c| c.peer_id == peer_id) {
                c.name = name.clone();
            }
            if let Some(c) = app.mdns_results.iter_mut().find(|c| c.peer_id == peer_id) {
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
        WriteEvent::EventResponse(response) => {
            match response.result {
                Ok(resp_type) => {
                    use p2pchat_types::api::UiClientEventResponseType;
                    match resp_type {
                        UiClientEventResponseType::LoadChatlogPage(messages) => {
                            if app.last_chatlog_req_id == Some(response.req_id) {
                                app.messages = messages;
                            }
                        }
                        UiClientEventResponseType::LoadFriends(friends) => {
                            for friend in friends {
                                if !app.contacts.iter().any(|c| c.peer_id == friend.peer_id) {
                                    app.contacts.push(friend);
                                }
                            }
                            if app.contact_list_state.selected().is_none()
                                && !app.contacts.is_empty()
                            {
                                app.contact_list_state.select(Some(0));
                                app.fetch_chatlog_for_selected();
                            }
                        }
                        UiClientEventResponseType::LoadPendingFriendRequests(pending) => {
                            app.pending_requests = pending;
                        }
                        UiClientEventResponseType::LoadIncomingFriendRequests(incoming) => {
                            app.incoming_requests = incoming;
                        }
                        UiClientEventResponseType::SearchUsername { peer_id } => {
                            let contact = Contact {
                                peer_id: peer_id.clone(),
                                name: app.friends_search_input.clone(),
                                discovery_type: DiscoveryType::Tracker,
                            };
                            if !app.search_results.iter().any(|c| c.peer_id == peer_id) {
                                app.search_results.push(contact);
                            }
                        }
                        _ => {}
                    }
                }
                Err(_err) => {}
            }
        }
        WriteEvent::CriticalFailure(_) => {
            app.should_quit = true;
        }
        WriteEvent::ReceiveFriendRequest => {
            app.send_request(UiClientEvent::LoadIncomingFriendRequests);
        }
        WriteEvent::DcutrConnection(event) => {
            match event.0 {
                Ok(success) => {
                    app.connection_status
                        .insert(success.peer_id.clone(), ConnectionType::Dcutr);
                    app.flush_pending_actions(&success.peer_id);
                }
                Err(_) => {}
            }
        }
        WriteEvent::ReceiveFriendRequestResponse { decision } => {
            app.send_request(UiClientEvent::LoadPendingFriendRequests);
            if decision {
                app.send_request(UiClientEvent::LoadFriends);
            }
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

    // Fetch initial data
    app.send_request(UiClientEvent::LoadFriends);
    app.send_request(UiClientEvent::LoadPendingFriendRequests);
    app.send_request(UiClientEvent::LoadIncomingFriendRequests);

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
            KeyCode::Char('j') | KeyCode::Down => {
                app.focused_list_next();
                if app.focus == Focus::ContactList {
                    app.fetch_chatlog_for_selected();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.focused_list_prev();
                if app.focus == Focus::ContactList {
                    app.fetch_chatlog_for_selected();
                }
            }
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
                        app.send_request(UiClientEvent::SearchUsername { username: query });
                    }
                } else {
                    app.send_chat_message();
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
                        app.fetch_chatlog_for_selected();
                    }
                }
            } else if app.selected_tab == Tab::Friends {
                // Check accept/deny button hits on incoming requests
                for (i, (accept_rect, deny_rect)) in app.incoming_button_areas.iter().enumerate() {
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
        MouseEventKind::ScrollDown => {
            app.focused_list_next();
            if app.focus == Focus::ContactList {
                app.fetch_chatlog_for_selected();
            }
        }
        MouseEventKind::ScrollUp => {
            app.focused_list_prev();
            if app.focus == Focus::ContactList {
                app.fetch_chatlog_for_selected();
            }
        }
        _ => {}
    }
}

fn rect_contains(rect: &ratatui::layout::Rect, col: u16, row: u16) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}
