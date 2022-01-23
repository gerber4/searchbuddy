- CLI tool. This CLI mocks the basic flow that the extension will use.
- Fixed set of chatroom service instances. No system for handling 
adding/removing instances.
- Messages are not saved. Once a user disconnects from a chatroom, they lose access
to the messages.
- There is no state associated with users. Users are assigned temporary IDs
when they connect to a chatroom.