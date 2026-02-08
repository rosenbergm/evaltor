-- Add down migration script here

alter table attempts
    drop column user_id;
