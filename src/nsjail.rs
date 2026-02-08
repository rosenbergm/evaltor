use std::path::PathBuf;
use std::process::Stdio;

use chrono::Utc;
use sqlx::SqlitePool;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::{process::Command, task::JoinHandle};
use uuid::Uuid;

use crate::models::{Runner, Test, TestType};

pub struct NSJailBlueprint {
    pub tests: PathBuf,

    pub memory_limit: i64,
    pub time_limit: i64,
    pub max_cpus: i64,
    pub disable_network: bool,
    pub mountpoint: PathBuf,

    pub command: String,
    pub write_stdin: bool,

    pub quiet: bool,
}

impl NSJailBlueprint {
    pub fn into_command(self) -> Command {
        let mut cmd = Command::new("sudo");

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if self.write_stdin {
            cmd.stdin(Stdio::piped());
        }

        cmd.args([
            "nsjail",
            "--mode",
            "o",
            "--chroot",
            "/",
            "--cwd",
            "/workspace",
            "--tmpfsmount",
            "/tmp",
            "--disable_clone_newuser",
        ]);

        cmd.arg("--rlimit_as").arg(self.memory_limit.to_string());
        cmd.arg("--time_limit").arg(self.time_limit.to_string());
        cmd.arg("--max_cpus").arg(self.max_cpus.to_string());
        cmd.arg("--bindmount_ro")
            .arg(format!("{}:/workspace", self.mountpoint.to_string_lossy()));

        if self.disable_network {
            cmd.arg("--disable_clone_newnet");
        }

        if self.quiet {
            cmd.arg("--quiet");
        }

        cmd.arg("--").args(self.command.split_ascii_whitespace());

        cmd
    }
}

pub struct Instance {
    blueprint: NSJailBlueprint,
}

impl Instance {
    pub const fn new(blueprint: NSJailBlueprint) -> Self {
        Self { blueprint }
    }

    pub fn spawn(self, db: SqlitePool, test_id: Uuid, attempt_id: Uuid) -> JoinHandle<()> {
        tokio::spawn(Self::worker(self.blueprint, db, test_id, attempt_id))
    }

    #[expect(clippy::too_many_lines)]
    async fn worker(
        blueprint: NSJailBlueprint,
        db: SqlitePool,
        test_id: Uuid,
        attempt_id: Uuid,
    ) -> () {
        let id = Uuid::new_v4();

        let runner = Runner {
            id,
            test_id,
            attempt_id,
            passed: false,
            points: 0,
            command_ran: blueprint.command.clone(),
            user_command_ran: blueprint.command.clone(),
            created_at: Utc::now().naive_utc(),
            finished_at: None,
            exit_code: None,
            stdout: None,
            stderr: None,
            expected_stderr: None,
            expected_stdout: None,
            memory_limit: blueprint.memory_limit,
            time_limit: blueprint.time_limit,
            max_cpus: blueprint.max_cpus,
            disable_network: blueprint.disable_network,
        };

        if let Err(err) = runner.insert_new(&db).await {
            eprintln!("Failed to insert runner into database for command: {err:?}");
            return;
        }

        let write_stdin = blueprint.write_stdin;
        let tests_path = blueprint.tests.clone();

        let mut cmd = blueprint.into_command();

        let Ok(mut child) = cmd.spawn() else {
            eprintln!("Failed to spawn nsjail for command");
            return;
        };

        let Ok(test) = sqlx::query_as!(
            Test,
            r#"SELECT
                id as "id: uuid::Uuid",
                assignment_id as "assignment_id: uuid::Uuid",
                type as "type_: TestType",
                name,
                description,
                points
            FROM tests WHERE id = ?"#,
            test_id
        )
        .fetch_one(&db)
        .await
        else {
            return;
        };

        if write_stdin {
            let stdin_path = tests_path
                .join(test.assignment_id.to_string())
                .join(test.id.to_string())
                .join("test.in");

            let stdin_content = fs::read(&stdin_path).await.unwrap_or_default();

            if let Some(mut stdin) = child.stdin.take() {
                _ = stdin.write_all(&stdin_content).await;
            }
        }

        let Ok(output) = child.wait_with_output().await else {
            eprintln!("Failed to wait for nsjail output for command",);
            return;
        };

        let expected_stdout;

        let passed = match test.type_ {
            TestType::Compare => {
                let expected_output = tests_path
                    .join(test.assignment_id.to_string())
                    .join(test.id.to_string())
                    .join("test.out");

                let Ok(expected_output) = fs::read(&expected_output).await else {
                    eprintln!("Failed to read expected output file at");
                    return;
                };

                let success = expected_output.trim_ascii() == output.stdout.trim_ascii();

                expected_stdout = Some(expected_output);

                success
            }
        };

        let Ok(runner) = sqlx::query_as!(
            Runner,
            r#"SELECT
                id as "id: uuid::Uuid",
                test_id as "test_id: uuid::Uuid",
                attempt_id as "attempt_id: uuid::Uuid",
                passed,
                points,
                command_ran,
                user_command_ran,
                created_at as "created_at: chrono::NaiveDateTime",
                finished_at as "finished_at: chrono::NaiveDateTime",
                exit_code,
                stdout,
                stderr,
                expected_stdout,
                expected_stderr,
                memory_limit,
                time_limit,
                max_cpus,
                disable_network
            FROM runners WHERE id = ?"#,
            id
        )
        .fetch_one(&db)
        .await
        else {
            eprintln!("Failed to fetch runner from database");
            return;
        };

        let points = if passed { test.points } else { 0 };

        if let Err(err) = runner
            .update_completed(
                &db,
                output.status.code(),
                output.stdout,
                output.stderr,
                expected_stdout,
                None,
                passed,
                points,
            )
            .await
        {
            eprintln!("Failed to update runner: {err:?}");
            return;
        }
    }
}
