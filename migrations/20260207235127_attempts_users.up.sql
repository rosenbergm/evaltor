-- Add up migration script here

alter table attempts
    add column user_id text not null references users(id) on delete cascade on update cascade;
