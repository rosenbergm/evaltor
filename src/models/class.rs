use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::Points;

#[derive(Serialize, Deserialize, Debug)]
pub struct Class {
    pub id: Uuid,

    pub creator_id: Uuid,

    pub name: String,
    pub description: String,
}

impl Class {
    pub async fn points_for_student(
        db: &SqlitePool,
        class_id: Uuid,
        student_id: Uuid,
    ) -> sqlx::Result<Points> {
        let row = sqlx::query!(
            r#"SELECT
                    COALESCE((
                        SELECT SUM(t.points)
                        FROM tests t
                        JOIN user_assignments ua ON ua.assignment_id = t.assignment_id
                        WHERE ua.class_id = ?1 AND ua.user_id = ?2
                    ), 0) AS "maximum!: i64",
                    COALESCE((
                        SELECT SUM(r.points)
                        FROM runners r
                        JOIN attempts a ON r.attempt_id = a.id AND a.user_id = ?2
                        JOIN user_assignments ua ON ua.assignment_id = a.assignment_id
                            AND ua.class_id = ?1
                        WHERE r.passed = true
                        AND NOT EXISTS (
                            SELECT 1 FROM attempts a2
                            WHERE a2.assignment_id = a.assignment_id
                            AND a2.user_id = a.user_id
                            AND a2.submitted_at > a.submitted_at
                        )
                    ), 0) AS "achieved!: i64""#,
            class_id,
            student_id,
        )
        .fetch_one(db)
        .await?;

        Ok(Points::new(row.maximum, row.achieved))
    }
}
