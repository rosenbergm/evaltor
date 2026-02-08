-- Add down migration script here

alter table runners
    drop column test_id;
