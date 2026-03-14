use num_enum::TryFromPrimitive;

#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
enum DiscoveryType {
    Mdns,
    Tracker,
}
#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
enum FriendRequestType {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, TryFromPrimitive)]
#[repr(u8)]
pub enum MessageStatus {
    ReceivedNotRead,
    ReceivedRead,
    SentOffNotRead,
    SentOffRead,
}
