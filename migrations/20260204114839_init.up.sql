-- Add up migration script here

create table assignments (
    id text not null primary key,

    name text not null,
    description text not null
);

create table classes (
    id text not null primary key,

    name text not null,
    description text not null
);

create table runners (
    id text not null primary key,

    command_ran text not null,
    user_command_ran text not null,
    created_at timestamp not null,

    finished_at timestamp,
    exit_code integer,
    stdout BLOB,
    stderr BLOB,

    memory_limit integer not null,
    time_limit integer not null,
    max_cpus integer not null,
    disable_network boolean not null
);

create table attempts (
    id text not null primary key,

    assignment_id text not null references assignments(id) on delete cascade on update cascade,
    class_id text not null references classes(id) on delete cascade on update cascade,
    runner_id text not null references runners(id) on delete cascade on update cascade,

    submitted_at timestamp not null
);

create table tests (
    id text not null primary key,

    name text not null,
    description text not null,

    type text not null,

    assignment_id text not null references assignments(id) on delete cascade on update cascade
);
