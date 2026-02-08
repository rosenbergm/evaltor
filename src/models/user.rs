use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub id: Uuid,
    pub google_sub: String,

    pub email: String,
    pub name: String,
}

impl User {
    pub async fn all(db: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            User,
            r#"SELECT
            id as "id: Uuid",
            google_sub,
            email,
            name
            FROM users
            "#
        )
        .fetch_all(db)
        .await
    }
}
