-- Add up migration script here

alter table tests
    add column points integer not null default 0;

alter table runners
    add column points integer not null default 0;
