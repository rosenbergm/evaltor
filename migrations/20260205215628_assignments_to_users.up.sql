-- Add up migration script here

create table user_assignments (
    id text not null primary key,

    assignment_id text not null references assignments(id) on delete cascade on update cascade,
    user_id text not null references users(id) on delete cascade on update cascade,
    class_id text not null references classes(id) on delete cascade on update cascade
);

create unique index assignment_users_assignment_id_user_id_class_id_idx on user_assignments(assignment_id, user_id, class_id);
