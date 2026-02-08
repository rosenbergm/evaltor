-- Add down migration script here

alter table runners drop column expected_stdout;
alter table runners drop column expected_stderr;
