pub mod types;
mod widgets;
use crossterm::event::KeyCode;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use futures::{FutureExt, StreamExt};
use libp2p::PeerId;
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode::Char;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::text::Text;
use ratatui::widgets::Paragraph;
use ratatui::widgets::{Block, List, ListDirection, ListState, Scrollbar, ScrollbarState};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use types::Message;

use crate::network::Client;
use crate::tui::types::Contact;

#[derive(Clone, Debug)]
pub enum Event {
    Init,
    Quit,
    Error,
    Closed,
    Tick,
    Render,
    FocusGained,
    FocusLost,
    Paste(String),
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    MessageReceived(Message),
    // TODO: do like refresh contact list from sqlite instead
    ReloadContacts(Vec<Contact>),
    AddContact(Contact),
    EditContact(Contact),
}
pub struct Tui {
    pub terminal: ratatui::DefaultTerminal,
    pub task: Option<JoinHandle<()>>,
    pub event_rx: UnboundedReceiver<Event>,
    pub event_tx: UnboundedSender<Event>,
    pub frame_rate: f64,
    pub tick_rate: f64,
}

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
struct Key;
impl Key {
    const LEFT: KeyCode = Char('h');
    const RIGHT: KeyCode = Char('l');
    const UP: KeyCode = Char('k');
    const DOWN: KeyCode = Char('j');
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
            (Key::LEFT, KeyModifiers::SHIFT) => {
                tracing::info!("changing selected tab");
                app.selected_tab.left();
                return;
            }
            (Key::RIGHT, KeyModifiers::SHIFT) => {
                tracing::info!("changing selected tab");
                app.selected_tab.right();
                return;
            }
            (Key::LEFT | Key::RIGHT | Key::UP | Key::DOWN, KeyModifiers::CONTROL) => {
                app.selected_tab = match key.code {
                    Key::LEFT => {
                        tracing::info!("Pressed LEFT + CONTROL");
                        if app.selected_tab == Tabline::Chatting(ContactPage::Chat) {
                            tracing::info!("Is on chat should transition to contacts");
                        }
                        tracing::info!("{:?}", app.selected_tab);

                        match app.selected_tab {
                            Tabline::Chatting(c) => Tabline::Chatting(c.left()),
                            Tabline::FriendRequests(f) => Tabline::FriendRequests(f),
                        }
                    }
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
        Event::EditContact(contact) => {
            // TODO: actually handle
            if let Some(idx) = app
                .contacts
                .iter()
                .position(|x| x.peer_id == contact.peer_id)
            {
                let c = app.contacts.get_mut(idx).expect("unreachable");
                *c = contact;
            }
            return;
        }
        Event::Init => {}
        _ => {}
    };
    match &app.selected_tab {
        Tabline::Chatting(contact) => match contact {
            ContactPage::ContactList => handle_contact_list(app, event),
            ContactPage::Chat => handle_chat(app, event).await,
            ContactPage::CallButton => handle_call_button(app, event),
        },
        Tabline::FriendRequests(fr) => match fr {
            FriendRequestPage::RequestList => handle_request_list(app, event),
            FriendRequestPage::Search => handle_search(app, event),
        },
    }
}
fn handle_contact_list(app: &mut App, event: Event) {
    if let Event::Key(key) = event {
        match key.code {
            Key::RIGHT => app.selected_tab = Tabline::Chatting(ContactPage::Chat),
            Key::UP => app.selected_contact.select_previous(),
            Key::DOWN | KeyCode::Enter => app.selected_contact.select_next(),
            _ => unimplemented!(),
        }
    }
}
async fn handle_chat(app: &mut App, event: Event) {
    if let Event::Key(key) = event {
        match key.code {
            KeyCode::Backspace => {
                app.buffer.pop();
            }
            KeyCode::Enter => {
                let receiver = app
                    .contacts
                    .get(app.selected_contact.selected().unwrap())
                    .unwrap();
                app.client
                    .send_message(receiver.peer_id, app.buffer.clone())
                    .await;
                // add the message to our chat log
                app.chat.push(Message {
                    sender: Contact {
                        peer_id: app.client.id,
                        name: "You".to_string(),
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
    unimplemented!();
}
trait MoveHorizontal {
    fn left(self) -> Self;
    fn right(self) -> Self;
}
trait MoveVertical {
    fn up(self) -> Self;
    fn down(self) -> Self;
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tabline {
    Chatting(ContactPage),
    FriendRequests(FriendRequestPage),
}
impl Default for Tabline {
    fn default() -> Self {
        Self::Chatting(ContactPage::default())
    }
}
impl MoveHorizontal for Tabline {
    fn left(self) -> Self {
        if let Self::FriendRequests(_) = self {
            return Self::Chatting(ContactPage::default());
        }
        self
    }
    fn right(self) -> Self {
        if let Self::Chatting(_) = self {
            return Self::FriendRequests(FriendRequestPage::default());
        }
        self
    }
}
impl MoveVertical for FriendRequestPage {
    fn up(self) -> Self {
        match self {
            Self::RequestList => Self::Search,
            Self::Search => Self::RequestList,
        }
    }
    fn down(self) -> Self {
        match self {
            Self::RequestList => Self::Search,
            Self::Search => Self::RequestList,
        }
    }
}
impl MoveHorizontal for ContactPage {
    fn left(self) -> Self {
        match self {
            Self::Chat => Self::ContactList,
            Self::CallButton => Self::ContactList,
            _ => self,
        }
    }
    fn right(self) -> Self {
        match self {
            Self::ContactList => Self::Chat,
            _ => self,
        }
    }
}
impl MoveVertical for ContactPage {
    fn up(self) -> Self {
        match self {
            Self::Chat => Self::CallButton,
            _ => self,
        }
    }
    fn down(self) -> Self {
        match self {
            Self::CallButton => Self::Chat,
            _ => self,
        }
    }
}
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
enum ContactPage {
    #[default]
    ContactList,
    Chat,
    CallButton,
}
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
enum FriendRequestPage {
    #[default]
    RequestList,
    Search,
}
fn ui(f: &mut Frame, app: &mut App) {
    match app.selected_tab {
        Tabline::Chatting(_) => {
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
        Tabline::FriendRequests(_) => {} // chat
    }
    // friend list
}
// App state
struct App {
    selected_tab: Tabline,
    selected_contact: ListState,
    contacts: Vec<Contact>,
    should_quit: bool,
    chat: Vec<Message>,
    buffer: String,
    client: Client,
    token: CancellationToken,
}
pub async fn run(client: Client, token: CancellationToken, mut tui: Tui) -> anyhow::Result<()> {
    // ratatui terminal
    tui.start();

    // application state
    let mut app = App {
        selected_tab: Tabline::default(),
        should_quit: false,
        client,
        contacts: vec![
            // Contact {
            //     name: "Mark".to_string(),
            //     peer_id: PeerId::random(),
            // },
            // "Zuckerlizard".to_string(),
        ],
        selected_contact: ListState::default().with_selected(Some(0)),
        chat: vec![
        //     Message {
        //     sender: Contact {
        //         peer_id: PeerId::random(),
        //         name: "Mark".to_string(),
        //     },
        //     content: "adadadada adadad".to_string(),
        //     id: uuid::Uuid::new_v4(),
        //     status: types::MessageStatus::ReceivedRead,
        // }
        ],
        buffer: String::new(),
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
