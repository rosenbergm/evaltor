use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
pub struct Attempt {
    pub id: Uuid,

    pub assignment_id: Uuid,
    pub user_id: Uuid,

    pub submitted_at: NaiveDateTime,
}
