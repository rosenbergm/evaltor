use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct Runner {
    pub id: Uuid,

    pub test_id: Uuid,
    pub attempt_id: Uuid,

    pub passed: bool,
    pub points: i64,

    pub command_ran: String,
    pub user_command_ran: String,
    pub created_at: NaiveDateTime,

    pub finished_at: Option<NaiveDateTime>,
    pub exit_code: Option<i64>,
    pub stdout: Option<Vec<u8>>,
    pub stderr: Option<Vec<u8>>,
    pub expected_stdout: Option<Vec<u8>>,
    pub expected_stderr: Option<Vec<u8>>,

    pub memory_limit: i64,
    pub time_limit: i64,
    pub max_cpus: i64,
    pub disable_network: bool,
}

impl Runner {
    pub async fn insert_new(self, pool: &SqlitePool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO runners (id, test_id, attempt_id, passed, points, command_ran, user_command_ran, created_at, memory_limit, time_limit, max_cpus, disable_network) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            self.id,
            self.test_id,
            self.attempt_id,
            self.passed,
            self.points,
            self.command_ran,
            self.user_command_ran,
            self.created_at,
            self.memory_limit,
            self.time_limit,
            self.max_cpus,
            self.disable_network,
        ).execute(pool).await?;

        Ok(())
    }

    #[expect(clippy::too_many_arguments)]
    pub async fn update_completed(
        self,
        pool: &SqlitePool,
        exit_code: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        expected_stdout: Option<Vec<u8>>,
        expected_stderr: Option<Vec<u8>>,
        passed: bool,
        points: i64,
    ) -> sqlx::Result<()> {
        let now = Utc::now().naive_utc();

        sqlx::query!(
            "UPDATE runners SET finished_at = ?, exit_code = ?, stdout = ?, stderr = ?, expected_stdout = ?, expected_stderr = ?, passed = ?, points = ? WHERE id = ?",
            now,
            exit_code,
            stdout,
            stderr,
            expected_stdout,
            expected_stderr,
            passed,
            points,
            self.id,
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
