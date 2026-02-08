-- Add down migration script here

alter table tests
    drop column points;

alter table runners
    drop column points;
