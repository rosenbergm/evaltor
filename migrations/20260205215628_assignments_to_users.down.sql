-- Add down migration script here

drop index assignment_users_assignment_id_user_id_class_id_idx;

drop table user_assignments;
