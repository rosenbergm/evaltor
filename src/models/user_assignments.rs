use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use uuid::Uuid;

use crate::models::assignment::Assignment;

#[expect(dead_code)]
pub struct UserAssignment {
    pub id: Uuid,

    pub user_id: Uuid,
    pub assignment_id: Uuid,
    pub class_id: Uuid,
}

impl UserAssignment {
    pub async fn assignments_for_user(
        db: &SqlitePool,
        user_id: Uuid,
        class_id: Uuid,
    ) -> sqlx::Result<Vec<Assignment>> {
        let assignment_ids = sqlx::query!(
            r#"SELECT
                assignment_id as "assignment_id: uuid::Uuid"
            FROM user_assignments WHERE user_id = ? AND class_id = ?"#,
            user_id,
            class_id
        )
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|record| record.assignment_id);

        let mut builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("SELECT * FROM assignments WHERE id IN ");

        builder.push_tuples(assignment_ids, |mut b, id| {
            b.push_bind(id);
        });

        builder.build_query_as::<Assignment>().fetch_all(db).await
    }

    pub async fn assign_to_student(
        db: &SqlitePool,
        user_id: Uuid,
        assignment_id: Uuid,
        class_id: Uuid,
    ) -> sqlx::Result<()> {
        let new_id = Uuid::new_v4();

        sqlx::query!(
            "INSERT INTO user_assignments (id, user_id, assignment_id, class_id) VALUES (?, ?, ?, ?)",
            new_id,
            user_id,
            assignment_id,
            class_id
        )
        .execute(db)
        .await
        .inspect_err(|e| {
            dbg!(e);
        })
        ?;

        Ok(())
    }
}
