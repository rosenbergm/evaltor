use askama::Template;
use chrono::NaiveDateTime;
use uuid::Uuid;

pub struct RunnerResult {
    pub test_name: String,
    pub finished_at: Option<NaiveDateTime>,
    pub passed: bool,
    pub stdout: Option<String>,
    pub expected_stdout: Option<String>,

    pub test_points: i64,
    pub runner_points: i64,
}

#[derive(Template)]
#[template(path = "partials/runners.html")]
pub struct RunnersPartial {
    pub attempt_id: Uuid,
    pub runners: Vec<RunnerResult>,

    pub total_test_points: i64,
    pub total_runner_points: i64,
}
