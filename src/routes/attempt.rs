use askama::Template;
use axum::{
    Router,
    extract::{Path, State},
    response::Html,
    routing::get,
};
use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth,
    state::EvaltorState,
    templates::{RunnerResult, RunnersPartial},
};

pub fn router() -> axum::Router<EvaltorState> {
    Router::new().route("/attempts/{id}/runners", get(get_runners))
}

async fn get_runners(
    _auth: auth::AuthUser,
    State(state): State<EvaltorState>,
    Path(attempt_id): Path<Uuid>,
) -> Result<Html<String>, StatusCode> {
    let mut total_test_points = 0;
    let mut total_runner_points = 0;

    let runners = sqlx::query!(
        r#"SELECT
            t.name as "test_name!",
            t.description as "test_description!",
            t.points as "test_points!",
            r.passed as "passed: bool",
            r.finished_at as "finished_at: chrono::NaiveDateTime",
            r.stdout as "stdout: Vec<u8>",
            r.expected_stdout as "expected_stdout: Vec<u8>",
            r.points as "runner_points!"
        FROM runners r
        JOIN tests t ON r.test_id = t.id
        WHERE r.attempt_id = ?
        ORDER BY t.name"#,
        attempt_id
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(|record| {
        total_test_points += record.test_points;
        total_runner_points += record.runner_points;

        RunnerResult {
            finished_at: record.finished_at,
            passed: record.passed,
            stdout: record.stdout.and_then(|out| String::from_utf8(out).ok()),
            expected_stdout: record
                .expected_stdout
                .and_then(|out| String::from_utf8(out).ok()),
            test_name: record.test_name,
            test_description: record.test_description,

            test_points: record.test_points,
            runner_points: record.runner_points,
        }
    })
    .collect();

    RunnersPartial {
        attempt_id,
        runners,

        total_test_points,
        total_runner_points,
    }
    .render()
    .map(Html)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
