CREATE TABLE chat (
      chatroom_id INT,
      ts TIMESTAMPTZ,
      content STRING NOT NULL,
      PRIMARY KEY(chatroom_id, ts)
)