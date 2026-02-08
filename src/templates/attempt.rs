use askama::Template;
use uuid::Uuid;

use crate::models::Attempt;

#[derive(Template)]
#[template(path = "partials/attempts.html")]
#[expect(dead_code)]
pub struct AttemptsPartial {
    pub assignment_id: Uuid,
    pub attempts: Vec<Attempt>,
}
