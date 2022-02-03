## Discovery
The purpose of this service to map terms to active instances of the chatroom service.
Chatrooms can dynamically join and leave the cluster and the mapping will be kept
up to date.

### Testing
The simplest method to test this service is to use a tool such as `httpie` or `xq`.

#### Register an instance:
Running `xh post :8081/register address=0.0.0.0:3001` will register a new instance
and return the instance_id for the instance. The instance will be considered active
for 10 seconds. To keep the instance from going inactive, run 
`watch xh post :8081/ping address=0.0.0.0:3000 instance_id:=-<instance_id>`. This
command will continuously ping the server to keep the instance alive.

#### Finding a chatroom:
Running `xh post :8081/chatroom term=<term>` will map the given term to the 
associated instance of the chatroom service. If the associated service ever
becomes inactive, this command will start returning a new instance.

#### Killing an instance:
Killing the watch command will simulate an instance dying and allow you to test
re-mapping.
