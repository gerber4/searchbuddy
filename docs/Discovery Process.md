1. Send the current search terms to the discovery service.
2. The discovery service sends requests to the chatroom discovery service.
Currently, this involves hashing each term and sending a request to each of the
relevant chatroom instances. The chatroom instances will send the number of users,
if any, in the rooms.
3. The discovery service will return the results of these requests.
4. The user will request the address for it to start/join a chatroom for one
of its search terms.
5. The user will send a request and create a WebSocket connection to join 
the chat room.
6. Once the user is finished, the user will close the WebSocket connection to
leave the chatroom.