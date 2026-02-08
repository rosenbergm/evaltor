-- Add up migration script here

alter table runners
    add column attempt_id text not null references attempts(id) on delete cascade on update cascade;
