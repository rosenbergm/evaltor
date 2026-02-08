-- Add down migration script here

alter table classes
    drop column creator_id;
