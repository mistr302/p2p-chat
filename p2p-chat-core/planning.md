# Planning for the application

## General

On start:
// TODO: when dht is integrated should look for messages/other information that couldve been sent when offline

While running:

## TUI

### Chat section

On application start:
The application fetches all known peers from its database, and shows contact list.
// TODO: when dht is integrated should look for messages/other information that couldve been sent when offline

While running:
When opening a conversation, the chatlog is loaded from sqlite

#### Upon receiving a message in network_events, tui event_tx sends Message

- would be great to implement a notification daemon
- creates a new message in sqlite
- adds a notification to the contact, indicating unread messages

#### Upon opening a chat

- read all messages, sending ReadMessage for each message from client_tx
- update states ReceivedMessageUnread to read in for all received messages in the sqlite table

#### Upon sending a message

- send the message using client_tx
- insert the message to sqlite

### Friend request section
// Name sending shouldnt be available on non mdns comm
