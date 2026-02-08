-- Add up migration script here

alter table runners
    add column test_id text not null references tests(id) on delete set null on update cascade;
