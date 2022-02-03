All messages are events that are either sent from the client to the server or
vice versa.

Server -> Client
- JOINED {user} - Sent when a user has connected.
- NEW_MESSAGE {idempotency, text, timestamp, user}
- NEW_USER {timestamp, user}
- USER_DISCONNECT {user}
- CHATS_FROM_TODAY_RESPONSE {messages[]}

Client -> Server
- NEW_MESSAGE {idempotency, text}
- CHATS_FROM_TODAY_REQUEST
- DISCONNECT