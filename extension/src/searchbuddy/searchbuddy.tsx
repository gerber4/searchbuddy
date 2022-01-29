import "./styles.css";
import * as React from "react";
import { render } from "react-dom";
import { useEffect, useReducer, useState } from "react";
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
    const [partyTime, setPartyTime] = useState(false);
    const size = useWindowSize();

    let content;

    switch (model.state.type) {
        case "Initial":
            content = <Initial dispatch={dispatch} />;
            break;
        case "SelectChatrooms":
            content = (
                <SelectChatrooms
                    dispatch={dispatch}
                    chatrooms={model.state.chatrooms}
                />
            );
            break;
        case "InChatroom":
            content = (
                <InChatroom
                    dispatch={dispatch}
                    chatroom={model.state.chatroom}
                />
            );
            break;
        case "Error":
            content = <p>{model.state.message}</p>;
            break;
    }

    let partyButton;
    if (size.width > 1000) {
        partyButton = (
            <button
                className="partyButton"
                onClick={() => {
                    setPartyTime(!partyTime);
                }}
            />
        );
    } else {
        partyButton = <React.Fragment />;
    }

    return (
        <div className={"mainContainer" + (partyTime ? " partyTime" : "")}>
            <div className="mainColumn">{content}</div>
            {partyButton}
        </div>
    );
};

interface Size {
    width: number;
    height: number;
}

function useWindowSize(): Size {
    const [windowSize, setWindowSize] = useState<Size>({
        width: 0,
        height: 0,
    });

    useEffect(() => {
        // Handler to call on window resize
        function handleResize() {
            // Set window width/height to state
            setWindowSize({
                width: window.innerWidth,
                height: window.innerHeight,
            });
        }

        // Add event listener
        window.addEventListener("resize", handleResize);

        // Call handler right away so state gets updated with initial window size
        handleResize();

        // Remove event listener on cleanup
        return () => window.removeEventListener("resize", handleResize);
    }, []);

    return windowSize;
}

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
