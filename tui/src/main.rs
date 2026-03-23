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
            KeyCode::Char('j') | KeyCode::Down => app.next_contact(),
            KeyCode::Char('k') | KeyCode::Up => app.previous_contact(),
            KeyCode::Char('l') | KeyCode::Right => app.focus = Focus::Chat,
            KeyCode::Char('h') | KeyCode::Left => app.focus = Focus::ContactList,
            KeyCode::Char('i') | KeyCode::Enter => {
                if app.focus == Focus::Chat {
                    app.input_mode = InputMode::Editing;
                }
            }
            _ => {}
        },
        InputMode::Editing => match key.code {
            KeyCode::Esc => app.input_mode = InputMode::Normal,
            KeyCode::Enter => {
                // TODO: send message via request_tx
                app.input.clear();
            }
            KeyCode::Backspace => {
                app.input.pop();
            }
            KeyCode::Char(c) => {
                app.input.push(c);
            }
            _ => {}
        },
    }
}

fn handle_mouse_event(app: &mut App, mouse: event::MouseEvent) {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let col = mouse.column;
            // Left ~30 columns is the contact list area
            if col < 30 {
                app.focus = Focus::ContactList;
                // Tabs take ~3 rows at top, status bar ~3 at bottom
                let row = mouse.row as usize;
                if row >= 4 {
                    let contact_idx = row.saturating_sub(4) + app.contact_list_state.offset();
                    if contact_idx < app.contacts.len() {
                        app.contact_list_state.select(Some(contact_idx));
                    }
                }
            } else {
                app.focus = Focus::Chat;
                app.input_mode = InputMode::Editing;
            }
        }
        MouseEventKind::ScrollDown => {
            if app.focus == Focus::ContactList {
                app.next_contact();
            }
        }
        MouseEventKind::ScrollUp => {
            if app.focus == Focus::ContactList {
                app.previous_contact();
            }
        }
        _ => {}
    }
}
