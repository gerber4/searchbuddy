import React from "react";
import type { Chatroom, Msg } from "./searchbuddy";

export const SelectChatrooms: React.FC<{
    dispatch: React.Dispatch<Msg>;
    chatrooms: Chatroom[];
}> = ({ dispatch, chatrooms }) => {
    return (
        <table>
            <thead>
                <tr>
                    <th>Term</th>
                    <th>Users</th>
                    <th>Actions</th>
                </tr>
            </thead>

            <tbody>
                <React.Fragment>
                    {chatrooms.map((chatroom: Chatroom, index) => {
                        return (
                            <Row
                                key={index}
                                dispatch={dispatch}
                                chatroom={chatroom}
                            />
                        );
                    })}
                </React.Fragment>
            </tbody>
        </table>
    );
};

const Row: React.FC<{ dispatch: React.Dispatch<Msg>; chatroom: Chatroom }> = ({
    dispatch,
    chatroom,
}) => {
    function onClickConnect(_event: React.MouseEvent) {
        dispatch({ type: "SelectChatroom", chatroom: chatroom });
    }

    return (
        <tr>
            <td>{chatroom.term}</td>
            <td>{chatroom.num_users}</td>
            <td>
                <button onClick={onClickConnect}>Connect</button>
            </td>
        </tr>
    );
};
