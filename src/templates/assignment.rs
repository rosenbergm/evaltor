use askama::Template;

use crate::{filters, models::Assignment};

#[derive(Template)]
#[template(path = "assignment.html")]
pub struct AssignmentPage {
    pub user_name: String,
    pub user_email: String,
    pub assignment: Assignment,
}
