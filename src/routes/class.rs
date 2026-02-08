use askama::Template;
use axum::{
    Form, Router,
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
};
use reqwest::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AssignmentPage, Points, auth, filters,
    models::{Assignment, Class, User, UserAssignment},
    state::EvaltorState,
};

pub fn router() -> axum::Router<EvaltorState> {
    Router::new()
        .route("/classes/{id}/assign", post(assign_to_student))
        .route("/classes/{class_id}/{assignment_id}", get(class_assignment))
        .route("/classes/{id}", get(get_class))
}

#[derive(Deserialize)]
struct AssignToStudentForm {
    user_id: Uuid,
    assignment_id: Uuid,
}

async fn assign_to_student(
    _auth: auth::AuthUser,
    State(state): State<EvaltorState>,
    Path(class_id): Path<Uuid>,
    Form(AssignToStudentForm {
        user_id,
        assignment_id,
    }): Form<AssignToStudentForm>,
) -> Result<Redirect, StatusCode> {
    UserAssignment::assign_to_student(&state.db_pool, user_id, assignment_id, class_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(&format!("/classes/{class_id}")))
}

async fn class_assignment(
    auth: auth::AuthUser,
    State(state): State<EvaltorState>,
    Path((class_id, assignment_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let assignment_id = sqlx::query!(
        r#"SELECT assignment_id as "id: uuid::Uuid" FROM user_assignments WHERE class_id = ? and assignment_id = ?"#,
        class_id,
        assignment_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let assignment = sqlx::query_as!(
        Assignment,
        r#"SELECT id as "id: uuid::Uuid", name, description FROM assignments WHERE id = ?"#,
        assignment_id.id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    dbg!(&assignment);

    AssignmentPage {
        user_name: auth.0.name,
        user_email: auth.0.email,
        assignment,
    }
    .render()
    .map(Html)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Template)]
#[template(path = "class.html")]
struct ClassPage {
    user_name: String,
    user_email: String,
    user_id: Uuid,
    class: Class,
    assignments: Vec<Assignment>,
    points: Points,
    all_users: Vec<User>,
    all_assignments: Vec<Assignment>,
}

async fn get_class(
    auth: auth::AuthUser,
    State(state): State<EvaltorState>,
    Path(class_id): Path<Uuid>,
) -> Result<Html<String>, StatusCode> {
    let class = sqlx::query_as!(
        Class,
        r#"SELECT id as "id: uuid::Uuid", creator_id as "creator_id: uuid::Uuid", name, description FROM classes WHERE id = ?"#,
        class_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let assignments = UserAssignment::assignments_for_user(&state.db_pool, auth.0.id, class_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let points = Class::points_for_student(&state.db_pool, class_id, auth.0.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let all_users = User::all(&state.db_pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let all_assignments = Assignment::all(&state.db_pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    ClassPage {
        user_name: auth.0.name,
        user_email: auth.0.email,
        user_id: auth.0.id,
        class,
        assignments,
        points,
        all_users,
        all_assignments,
    }
    .render()
    .map(Html)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
