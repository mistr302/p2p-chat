use num_enum::TryFromPrimitive;

#[derive(Debug, PartialEq, Clone, TryFromPrimitive)]
#[repr(u8)]
pub enum DiscoveryType {
    Mdns = 0,
    Tracker = 1,
    You = 2,
}
#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
pub enum FriendRequestType {
    Incoming = 0,
    Outgoing = 1,
}

#[derive(Debug, Clone, TryFromPrimitive)]
#[repr(u8)]
pub enum MessageStatus {
    ReceivedNotRead = 0,
    ReceivedRead = 1,
    SentOffNotRead = 2,
    SentOffRead = 3,
}
