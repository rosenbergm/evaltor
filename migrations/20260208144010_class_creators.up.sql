-- Add up migration script here

alter table classes
    add column creator_id text not null references users(id) on delete cascade on update cascade;
