-- Add up migration script here

alter table runners
    add column passed boolean not null default false;
