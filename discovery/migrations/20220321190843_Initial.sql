-- Add migration script here

CREATE TABLE instance (
    region STRING,
    address STRING,
    instance_id INT NOT NULL,
    last_accessed TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (region, address)
);

CREATE TABLE chatroom (
    term STRING PRIMARY KEY,
    address STRING NOT NULL,
    instance_id INT NOT NULL
);