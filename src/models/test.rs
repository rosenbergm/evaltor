use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct Test {
    pub id: Uuid,

    pub name: String,
    pub description: String,

    pub points: i64,

    #[serde(rename = "type")]
    pub type_: TestType,

    pub assignment_id: Uuid,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug)]
#[sqlx(rename_all = "snake_case")]
pub enum TestType {
    Compare,
}
