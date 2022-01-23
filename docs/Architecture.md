## Goals
- Provide the tools to find users searching for similar terms and a chatroom
to talk with these people.
- Google Scale - Build an application that could scale up to service 
Google's entire user-base.

## Services

### Discovery Service
The purpose of this service is to help users discover other users.
This service will provide the routing information required for users to join
a shared chatroom.

### Chatroom Registry Service
The purpose of this service is to maintain a registry of the instances
of the chatroom service. This service will provide an API that will allow for
the even distribution of active conversations across the cluster of 
chatroom instances.

To simplify the initial implementation the set of chatroom instances will
be fixed.

### Chatroom Service
The purpose of this service is to manage the events and state of a chatroom.