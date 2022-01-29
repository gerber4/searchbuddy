import "./styles.css";
import * as React from "react";
import { render } from "react-dom";
import { useEffect, useReducer } from "react";
import { SelectChatrooms } from "./selectChatrooms";
import { InChatroom } from "./chatroom";
import { Initial } from "./initial";

export interface Chatroom {
    chatroom_id: number;
    num_users: number;
    online: boolean;
    term: string;
    url: string;
}

type State =
    | { type: "Initial" }
    | { type: "SelectChatrooms"; chatrooms: Chatroom[] }
    | { type: "InChatroom"; chatroom: Chatroom }
    | { type: "Error"; message: string };

interface Model {
    state: State;
}

export type Msg =
    | { type: "ChatroomsResponse"; chatrooms: Chatroom[] }
    | { type: "SelectChatroom"; chatroom: Chatroom }
    | { type: "Error"; message: string };

function reducer(model: Model, msg: Msg): Model {
    switch (msg.type) {
        case "ChatroomsResponse":
            return {
                ...model,
                state: { type: "SelectChatrooms", chatrooms: msg.chatrooms },
            };
        case "SelectChatroom":
            return {
                ...model,
                state: {
                    type: "InChatroom",
                    chatroom: msg.chatroom,
                },
            };
        case "Error":
            return {
                ...model,
                state: {
                    type: "Error",
                    message: msg.message,
                },
            };
    }
}

const App: React.FC<Model> = (initialModel: Model) => {
    const [model, dispatch] = useReducer(reducer, initialModel);

    switch (model.state.type) {
        case "Initial":
            return (
                <div className="mainContainer">
                    <div className="mainColumn">
                        <Initial dispatch={dispatch} />
                    </div>
                </div>
            );
        case "SelectChatrooms":
            return (
                <div className="mainContainer">
                    <div className="mainColumn">
                        <SelectChatrooms
                            dispatch={dispatch}
                            chatrooms={model.state.chatrooms}
                        />
                    </div>
                </div>
            );
        case "InChatroom":
            return (
                <div className="mainContainer">
                    <div className="mainColumn">
                        <InChatroom
                            dispatch={dispatch}
                            chatroom={model.state.chatroom}
                        />
                    </div>
                </div>
            );
        case "Error":
            return (
                <div className="mainContainer">
                    <div className="mainColumn">{model.state.message}</div>
                </div>
            );
    }
};

function main() {
    render(
        <App
            {...{
                state: { type: "Initial" },
            }}
        />,
        document.getElementById("searchbuddy"),
    );
}

main();
