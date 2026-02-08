use askama::Template;
use axum::{
    Router,
    body::Bytes,
    extract::{Path, State},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use axum_typed_multipart::{FieldData, TryFromMultipart, TypedMultipart};
use reqwest::StatusCode;
use tokio::fs;
use uuid::Uuid;

use crate::{
    auth,
    models::{Assignment, Attempt, Test, TestType},
    nsjail::NSJailBlueprint,
    state::EvaltorState,
    templates::{AssignmentPage, AttemptsPartial},
};

pub fn router() -> axum::Router<EvaltorState> {
    Router::new()
        .route("/assignments/{id}", get(assignment))
        .route("/assignments/{id}/attempts", get(get_attempts))
        .route("/assignments/{id}/attempts", post(post_attempt))
}

async fn assignment(
    auth: auth::AuthUser,
    State(state): State<EvaltorState>,
    Path(assignment_id): Path<Uuid>,
) -> Result<Html<String>, StatusCode> {
    let assignment = sqlx::query_as!(
        Assignment,
        r#"SELECT id as "id: uuid::Uuid", name, description FROM assignments WHERE id = ?"#,
        assignment_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    AssignmentPage {
        user_name: auth.name.clone(),
        user_email: auth.email.clone(),
        assignment,
    }
    .render()
    .map(Html)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_attempts(
    _auth: auth::AuthUser,
    State(state): State<EvaltorState>,
    Path(assignment_id): Path<Uuid>,
) -> Result<Html<String>, StatusCode> {
    let attempts = sqlx::query_as!(
        Attempt,
        r#"SELECT
                id as "id: uuid::Uuid",
                assignment_id as "assignment_id: uuid::Uuid",
                user_id as "user_id: uuid::Uuid",
                submitted_at as "submitted_at: chrono::NaiveDateTime"
                FROM attempts WHERE assignment_id = ? ORDER BY submitted_at DESC"#,
        assignment_id
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    AttemptsPartial {
        assignment_id,
        attempts,
    }
    .render()
    .map(Html)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Debug, TryFromMultipart)]
pub struct PostAssignmentForm {
    pub assignment_id: Uuid,
    #[form_data(limit = "10MiB")]
    pub program: FieldData<Bytes>,
}

async fn post_attempt(
    auth: auth::AuthUser,
    state: State<EvaltorState>,
    TypedMultipart(PostAssignmentForm {
        assignment_id,
        program,
    }): TypedMultipart<PostAssignmentForm>,
) -> impl IntoResponse {
    let attempt_id = Uuid::new_v4();

    let attempt = Attempt {
        id: attempt_id,
        assignment_id,
        user_id: auth.id,
        submitted_at: chrono::Utc::now().naive_utc(),
    };

    sqlx::query!(
        "INSERT INTO attempts (id, assignment_id, user_id, submitted_at) VALUES (?, ?, ?, ?)",
        attempt.id,
        attempt.assignment_id,
        auth.id,
        attempt.submitted_at,
    )
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tests = sqlx::query_as!(
        Test,
        r#"SELECT
            id as "id: uuid::Uuid",
            name,
            description,
            type as "type_: TestType",
            assignment_id as "assignment_id: uuid::Uuid",
            points
        FROM tests WHERE assignment_id = ?"#,
        assignment_id
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mountpoint = state
        .config
        .submissions
        .join(assignment_id.to_string())
        .join(auth.id.to_string())
        .join(attempt_id.to_string());

    dbg!(&mountpoint);

    fs::create_dir_all(&mountpoint)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    fs::write(mountpoint.join("main.py"), program.contents)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for test in tests {
        let blueprint = NSJailBlueprint {
            tests: state.config.tests.clone(),
            memory_limit: 512,
            time_limit: 5,
            max_cpus: 1,
            disable_network: true,
            mountpoint: mountpoint.clone(),
            command: "/usr/bin/python3 main.py".to_owned(),
            write_stdin: true,
            quiet: true,
        };

        state
            .runner_manager
            .run_from_blueprint(blueprint, test.id, attempt.id);
    }

    Ok::<_, StatusCode>((
        StatusCode::SEE_OTHER,
        [("Location", format!("/assignments/{assignment_id}/attempts"))],
    ))
}
