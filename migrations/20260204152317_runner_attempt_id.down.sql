-- Add down migration script here

alter table runners
    drop column attempt_id;
