-- Add up migration script here

CREATE TABLE users (
  id TEXT NOT NULL PRIMARY KEY,
  google_sub TEXT NOT NULL UNIQUE,
  email TEXT NOT NULL,
  name TEXT NOT NULL
);
