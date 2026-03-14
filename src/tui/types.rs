pub use crate::db::types::MessageStatus;
use crate::network::Client;
use crossterm::event::KeyCode;
use libp2p::PeerId;
use ratatui::crossterm::event::KeyCode::Char;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};
use ratatui::widgets::ListState;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
pub(crate) struct App {
    pub selected_tab: Tabline,
    pub selected_contact: ListState,
    pub contacts: Vec<Contact>,
    pub should_quit: bool,
    pub chat: Vec<Message>,
    pub buffer: String,
    pub client: Client,
    pub token: CancellationToken,
    pub friend_search_buffer: String,
    pub friend_search_results: Vec<Contact>,
    pub incoming_requests: Vec<Contact>,
    pub selected_incoming_request: ListState,
    pub selected_search_result: ListState,
}
#[derive(Debug, Clone)]
pub struct Message {
    pub content: String,
    pub id: uuid::Uuid,
    pub sender: Contact,
    pub status: MessageStatus,
    // TODO: date
}
#[derive(Debug, Clone, PartialEq)]
pub struct Contact {
    pub peer_id: PeerId,
    pub name: String,
}
pub trait MoveHorizontal {
    fn left(self) -> Self;
    fn right(self) -> Self;
}
pub(crate) trait MoveVertical {
    fn up(self) -> Self;
    fn down(self) -> Self;
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tabline {
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
        if let Self::Chatting(_) = self {
            return self;
        }
        Self::Chatting(ContactPage::default())
    }
    fn right(self) -> Self {
        if let Self::FriendRequests(_) = self {
            return self;
        }
        Self::FriendRequests(FriendRequestPage::default())
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
pub enum ContactPage {
    #[default]
    ContactList,
    Chat,
    CallButton,
}
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum FriendRequestPage {
    #[default]
    Search,
    RequestList,
    // ResultList,
}
pub(crate) struct Key;
impl Key {
    pub const LEFT: KeyCode = Char('h');
    pub const RIGHT: KeyCode = Char('l');
    pub const UP: KeyCode = Char('k');
    pub const DOWN: KeyCode = Char('j');
}
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
    SearchResult(Vec<Contact>),
    LoadFriendRequests(Vec<Contact>),
}
pub struct Tui {
    pub terminal: ratatui::DefaultTerminal,
    pub task: Option<JoinHandle<()>>,
    pub event_rx: UnboundedReceiver<Event>,
    pub event_tx: UnboundedSender<Event>,
    pub frame_rate: f64,
    pub tick_rate: f64,
}
