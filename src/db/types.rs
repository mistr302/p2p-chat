enum DiscoveryType {
    Mdns,
    Tracker,
}
enum FriendRequestType {
    Incoming,
    Outgoing,
}
#[derive(Debug, Clone)]
pub enum MessageStatus {
    ReceivedNotRead,
    ReceivedRead,
    SentOffNotRead,
    SentOffRead,
}
