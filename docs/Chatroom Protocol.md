All messages are events that are either sent from the client to the server or
vice versa.

Server -> Client
- JOINED {user} - Sent when a user has connected.
- NEW_MESSAGE {idempotency, text, timestamp, user}
- NEW_USER {timestamp, user}
- USER_DISCONNECT {user}
- RANGE_RESPONSE {messages[]}

Client -> Server
- NEW_MESSAGE {idempotency, text}
- RANGE_REQUEST {start_timestamp?, end_timestamp?}
- DISCONNECT