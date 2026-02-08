use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
pub struct Assignment {
    pub id: Uuid,

    pub name: String,
    pub description: String,
}

impl Assignment {
    pub async fn all(db: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            Assignment,
            r#"SELECT
            id as "id: Uuid",
            name,
            description
            FROM assignments
            "#
        )
        .fetch_all(db)
        .await
    }
}
