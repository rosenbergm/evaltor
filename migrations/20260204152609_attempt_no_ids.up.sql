-- Add up migration script here

alter table attempts
    drop column runner_id;
alter table attempts
    drop column class_id;
