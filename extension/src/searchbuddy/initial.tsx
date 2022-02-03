import * as React from "react";
import { useEffect } from "react";
import type { Msg } from "./searchbuddy";

export const Initial: React.FC<{
    dispatch: React.Dispatch<Msg>;
}> = ({ dispatch }) => {
    useEffect(() => {
        let params = new URLSearchParams(window.location.search);
        let terms = params.get("terms");

        if (terms == null) {
            terms = "searchbuddy"
        } else {
            terms = terms + " searchbuddy"
        }

        // let url = `http://localhost:8080/chatrooms?search=${terms}`;
        let url = `http://searchbuddy.gerber.website:8080/chatrooms?search=${terms}`;

        fetch(url).then(async (response) => {
            if (response.ok && response.status == 200) {
                dispatch({
                    type: "ChatroomsResponse",
                    chatrooms: await response.json(),
                });
            } else {
                dispatch({
                    type: "Error",
                    message:
                        "Couldn't communicate with the searchbuddy server.",
                });
            }
        });
    }, []);

    return <div>Connecting to server...</div>;
};
