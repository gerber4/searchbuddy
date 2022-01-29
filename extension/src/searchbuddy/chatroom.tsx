import React, { useEffect, useReducer, useRef, useState } from "react";
import type { Chatroom, Msg } from "./searchbuddy";

type ClientToServerMessage =
    | { type: "Join"; chatroom_id: number }
    | { type: "NewMessage"; content: string }
    | { type: "ChatsFromTodayRequest" };

type ServerToClientMessage =
    | { type: "Joined"; chatroom_id: number }
    | { type: "NewUser"; user_id: number }
    | { type: "UserDisconnected"; user_id: number }
    | { type: "NewMessage"; content: string }
    | { type: "ChatsFromTodayResponse"; messages: string[] };

interface ChatroomModel {
    connected: boolean;
    messages: string[];
}

type ChatroomMsg =
    | { type: "Connected" }
    | { type: "Disconnected" }
    | { type: "Joined"; chatroom_id: number }
    | { type: "NewUser"; user_id: number }
    | { type: "UserDisconnected"; user_id: number }
    | { type: "NewMessage"; content: string }
    | { type: "ChatsFromTodayResponse"; messages: string[] };

function reducer(model: ChatroomModel, msg: ChatroomMsg): ChatroomModel {
    switch (msg.type) {
        case "Connected":
            return {
                ...model,
                connected: true,
            };
        case "Disconnected":
            return {
                ...model,
                connected: false,
            };
        case "Joined":
            model.messages.push("Joined chatroom!");
            return { ...model };
        case "NewUser":
            model.messages.push(`User with id ${msg.user_id} joined chatroom!`);
            return { ...model };
        case "UserDisconnected":
            model.messages.push(`User with id ${msg.user_id} left chatroom!`);
            return { ...model };
        case "NewMessage":
            model.messages.push(msg["content"]);
            return { ...model };
        case "ChatsFromTodayResponse":
            model.messages.push(...msg.messages);
            return { ...model };
    }
}

export const InChatroom: React.FC<{
    dispatch: React.Dispatch<Msg>;
    chatroom: Chatroom;
}> = ({ dispatch, chatroom }) => {
    const [chatroomModel, chatroomDispatch] = useReducer(reducer, {
        connected: false,
        messages: [],
    });

    const [input, setInput] = useState("");

    const ws = useRef<WebSocket | null>(null);

    const sendMessage = async function () {
        if (input === "") {
            return;
        }

        let websocket = ws.current;
        if (websocket != null) {
            let message: ClientToServerMessage = {
                type: "NewMessage",
                content: input,
            };
            websocket.send(JSON.stringify(message));
            setInput("");
        }
    };

    useEffect(() => {
        const input = document.querySelectorAll('div[class="messages"]');
        const messages: Element = input.item(0);
        if (messages != null) {
            messages.scrollTo(0, messages.scrollHeight);
        }
    }, [chatroomModel]);

    useEffect(() => {
        const websocket = new WebSocket(chatroom.url);

        ws.current = websocket;

        websocket.onopen = () => {
            chatroomDispatch({ type: "Connected" });
            let message: ClientToServerMessage = {
                type: "Join",
                chatroom_id: chatroom.chatroom_id,
            };
            websocket.send(JSON.stringify(message));
            message = { type: "ChatsFromTodayRequest" };
            websocket.send(JSON.stringify(message));
        };
        websocket.onclose = () => {
            chatroomDispatch({ type: "Disconnected" });
        };
        websocket.onmessage = (event) => {
            let message: ServerToClientMessage = JSON.parse(event.data);
            switch (message.type) {
                case "Joined":
                case "NewUser":
                case "UserDisconnected":
                case "NewMessage":
                case "ChatsFromTodayResponse":
                    chatroomDispatch(message);
                    return;
                default:
                    dispatch({
                        type: "Error",
                        message: "An invalid message was received from server.",
                    });
            }
        };
        websocket.onerror = () => {
            dispatch({
                type: "Error",
                message: "An error occurred while communicating with chatroom.",
            });
        };
        return () => {
            websocket.close();
        };
    }, []);

    if (chatroomModel.connected) {
        return (
            <React.Fragment>
                <div className="messages">
                    {chatroomModel.messages.map((message, index) => {
                        return <div key={index}>{message}</div>;
                    })}
                </div>
                <div className="inputs">
                    <input
                        className="inputs_text"
                        type="text"
                        value={input}
                        onChange={(event) => {
                            setInput(event.target.value);
                        }}
                        onKeyUp={(event) => {
                            if (event.key === "Enter") {
                                sendMessage().catch((err) => {
                                    console.log(err);
                                });
                            }
                        }}
                    />
                    <button type="button" onClick={sendMessage}>
                        Send
                    </button>
                </div>
            </React.Fragment>
        );
    } else {
        return <div>Connecting to chatroom...</div>;
    }
};
