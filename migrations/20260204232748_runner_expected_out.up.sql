-- Add up migration script here

alter table runners add column expected_stdout BLOB;
alter table runners add column expected_stderr BLOB;
