#![deny(
    clippy::as_conversions,
    clippy::expect_used,
    clippy::future_not_send,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::pedantic,
    clippy::string_slice,
    clippy::todo,
    clippy::unwrap_used,
    unsafe_code
)]
#![allow(
    clippy::manual_non_exhaustive,
    clippy::missing_errors_doc,
    clippy::module_inception,
    clippy::module_name_repetitions,
    clippy::needless_return,
    clippy::single_match_else,
    clippy::multiple_crate_versions
)]

use std::io;

use askama::Template;
use axum::{
    Router,
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
};

pub use args::EvaltorArgs;
use axum_typed_multipart::{FieldData, TryFromMultipart, TypedMultipart};
use chrono::NaiveDateTime;
use sqlx::SqlitePool;
use tokio::fs;
use tower_sessions::{Expiry, SessionManagerLayer, cookie::time::Duration};
use tower_sessions_sqlx_store::SqliteStore;
use uuid::Uuid;

use crate::{
    models::{Assignment, Attempt, Class, Test, TestType},
    nsjail::NSJailBlueprint,
    runner_manager::RunnerManager,
    state::EvaltorState,
};

pub use points::Points;

mod args;
mod auth;
pub mod filters;
mod models;
mod nsjail;
mod points;
mod routes;
mod runner_manager;
mod state;

pub async fn server(args: EvaltorArgs) -> Result<Router, io::Error> {
    let db_pool = SqlitePool::connect("sqlite:data.db")
        .await
        .map_err(io::Error::other)?;

    // make_test_data(&db_pool, &args).await;

    let oidc_client = auth::build_oidc_client(
        args.hostname.clone(),
        args.google_client_id.clone(),
        args.google_client_secret.clone(),
    )
    .await?;

    let session_store = SqliteStore::new(db_pool.clone());
    session_store.migrate().await.map_err(io::Error::other)?;
    let session_layer = SessionManagerLayer::new(session_store)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(7)));

    let runner_manager = RunnerManager::new(db_pool.clone());

    let state = EvaltorState {
        db_pool,
        runner_manager,
        oidc_client,
        config: args,
    };

    let router = Router::new()
        .route("/", get(index))
        .route("/assignments/{id}", get(assignment))
        .route("/assignments/{id}/attempts", get(get_attempts))
        .route("/assignments/{id}/attempts", post(post_attempt))
        .route("/attempts/{id}/runners", get(get_runners))
        .merge(routes::class::router())
        .merge(auth::auth_router())
        .layer(session_layer)
        .with_state(state);

    Ok(router)
}

#[derive(Template)]
#[template(path = "assignment.html")]
struct ClassAssignmentPage {
    user_name: String,
    user_email: String,
    assignment: Assignment,
}

#[derive(Template)]
#[template(path = "assignment.html")]
struct AssignmentPage {
    user_name: String,
    user_email: String,
    assignment: Assignment,
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

#[derive(Debug, Template)]
#[template(path = "index.html")]
struct IndexPage {
    user_name: String,
    user_email: String,
    classes: Vec<Class>,
}

#[derive(Template)]
#[template(path = "partials/attempts.html")]
#[expect(dead_code)]
struct AttemptsPartial {
    assignment_id: Uuid,
    attempts: Vec<Attempt>,
}

#[derive(Debug, TryFromMultipart)]
pub struct PostAssignmentForm {
    assignment_id: Uuid,
    #[form_data(limit = "10MiB")]
    program: FieldData<Bytes>,
}

async fn index(auth: auth::AuthUser, State(state): State<EvaltorState>) -> impl IntoResponse {
    let classes = sqlx::query_as!(
        Class,
        r#"SELECT id as "id: uuid::Uuid", creator_id as "creator_id: uuid::Uuid", name, description FROM classes"#
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        dbg!(e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    IndexPage {
        user_name: auth.name.clone(),
        user_email: auth.email.clone(),
        classes,
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

struct RunnerResult {
    test_name: String,
    finished_at: Option<NaiveDateTime>,
    passed: bool,
    stdout: Option<String>,
    expected_stdout: Option<String>,

    test_points: i64,
    runner_points: i64,
}

#[derive(Template)]
#[template(path = "partials/runners.html")]
struct RunnersPartial {
    attempt_id: Uuid,
    runners: Vec<RunnerResult>,

    total_test_points: i64,
    total_runner_points: i64,
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

#[expect(clippy::expect_used)]
#[expect(clippy::unwrap_used)]
#[expect(dead_code)]
async fn make_test_data(db: &SqlitePool, args: &EvaltorArgs) {
    let user_id = Uuid::parse_str("1b20315c-cacb-45aa-8436-30f18bd5fa35").unwrap();

    let cid = Uuid::parse_str("05979590-4c3c-4ad8-a3b8-12a1422bebb2").unwrap();
    let class = Class {
        id: cid,
        creator_id: user_id,
        name: "PgU 25/26".to_string(),
        description: "Seminar uvodu do programovani 2025/26".to_string(),
    };

    sqlx::query!(
        "INSERT INTO classes (id, creator_id, name, description) VALUES (?, ?, ?, ?)",
        class.id,
        class.creator_id,
        class.name,
        class.description,
    )
    .execute(db)
    .await
    .expect("Failed to insert test class");

    let aid = Uuid::parse_str("949807b4-226b-4803-8058-c751c930220e").unwrap();
    let assignment = Assignment {
        id: aid,
        name: "Prvocisla".to_string(),
        description: r"
        Napiste program, ktery zjisti, zda je zadane cislo prvocislem.

        Pokud je cislo prvocislem, program vypise `YES` a skonci. Jinak vypise `NO`.
        "
        .to_owned(),
    };

    sqlx::query!(
        "INSERT INTO assignments (id, name, description) VALUES (?, ?, ?)",
        assignment.id,
        assignment.name,
        assignment.description,
    )
    .execute(db)
    .await
    .expect("Failed to insert test assignment");

    // ---

    let t1id = uuid::Uuid::new_v4();
    println!("Inserting easy test with id: {t1id}");
    let test = Test {
        id: t1id,
        name: "Jednicka".to_string(),
        description: "Testuje spravnost vystupu".to_string(),
        type_: TestType::Compare,
        assignment_id: assignment.id,
        points: 1,
    };

    sqlx::query!(
        "INSERT INTO tests (id, name, description, type, assignment_id) VALUES (?, ?, ?, ?, ?)",
        test.id,
        test.name,
        test.description,
        test.type_,
        test.assignment_id,
    )
    .execute(db)
    .await
    .expect("Failed to insert test data");

    let t2id = uuid::Uuid::new_v4();
    println!("Inserting medium test with id: {t2id}");
    let test = Test {
        id: t2id,
        name: "Ostatni".to_string(),
        description: "Testuje ostatni prvocisla".to_string(),
        type_: TestType::Compare,
        assignment_id: assignment.id,
        points: 1,
    };

    sqlx::query!(
        "INSERT INTO tests (id, name, description, type, assignment_id) VALUES (?, ?, ?, ?, ?)",
        test.id,
        test.name,
        test.description,
        test.type_,
        test.assignment_id,
    )
    .execute(db)
    .await
    .expect("Failed to insert test data");

    // ---

    fs::create_dir(args.tests.clone())
        .await
        .expect("cannot create tests dir");

    fs::create_dir(format!("{}/{}", args.tests.to_string_lossy(), aid))
        .await
        .expect("cannot create assignment dir");

    fs::create_dir(format!("{}/{}/{}", args.tests.to_string_lossy(), aid, t1id))
        .await
        .expect("cannot create test 1 dir");

    fs::create_dir(format!("{}/{}/{}", args.tests.to_string_lossy(), aid, t2id))
        .await
        .expect("cannot create test 2 dir");
}
