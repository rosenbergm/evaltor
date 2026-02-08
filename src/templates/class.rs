use askama::Template;
use uuid::Uuid;

use crate::{
    Points, filters,
    models::{Assignment, Class, User},
};

#[derive(Template)]
#[template(path = "class.html")]
pub struct ClassPage {
    pub user_name: String,
    pub user_email: String,
    pub user_id: Uuid,
    pub class: Class,
    pub assignments: Vec<Assignment>,
    pub points: Points,
    pub all_users: Vec<User>,
    pub all_assignments: Vec<Assignment>,
}
