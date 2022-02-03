1. Send the current search terms to the discovery service.
2. The discovery service sends requests to the chatroom discovery service. The discovery service
either find or creates a mapping from the given term to an instance.
3. The discovery service will return the results of these requests.
4. The user will request the address for it to start/join a chatroom for one
of its search terms.
5. The user will send a request and create a WebSocket connection to join 
the chat room.
6. Once the user is finished, the user will close the WebSocket connection to
leave the chatroom.