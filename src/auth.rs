use std::{io, ops::Deref};

use askama::Template;
use axum::{
    Router,
    extract::{FromRequestParts, Query, State},
    http::{StatusCode, request::Parts},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use openidconnect::{
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet,
    EndpointNotSet, EndpointSet, IssuerUrl, Nonce, RedirectUrl, Scope, TokenResponse,
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
};
use serde::Deserialize;
use tower_sessions::Session;
use uuid::Uuid;

use crate::{models::User, state::EvaltorState};

const GOOGLE_ISSUER_URL: &str = "https://accounts.google.com";

pub type DiscoveredClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

pub struct AuthUser(pub User);

impl Deref for AuthUser {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequestParts<EvaltorState> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &EvaltorState,
    ) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(IntoResponse::into_response)?;

        let user_id = session
            .get::<Uuid>("user_id")
            .await
            .inspect_err(|e| {
                dbg!(e);
            })
            .map_err(|_| Redirect::to("/login").into_response())?
            .ok_or_else(|| Redirect::to("/login").into_response())?;

        let user = sqlx::query_as!(
            User,
            r#"SELECT id as "id: uuid::Uuid", google_sub, email, name FROM users WHERE id = ?"#,
            user_id
        )
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|_| Redirect::to("/login").into_response())?
        .ok_or_else(|| Redirect::to("/login").into_response())?;

        Ok(Self(user))
    }
}

pub async fn build_oidc_client(
    hostname: String,
    client_id: String,
    client_secret: String,
) -> Result<DiscoveredClient, io::Error> {
    let issuer_url = IssuerUrl::new(GOOGLE_ISSUER_URL.to_owned()).map_err(io::Error::other)?;

    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(io::Error::other)?;

    let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client)
        .await
        .map_err(|e| io::Error::other(e.to_string()))?;

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
    )
    .set_redirect_uri(
        RedirectUrl::new(format!("{hostname}auth/callback")).map_err(io::Error::other)?,
    );

    Ok(client)
}

#[derive(Deserialize)]
struct AuthCallbackParams {
    code: String,
    state: String,
}

async fn google_login(
    State(state): State<EvaltorState>,
    session: Session,
) -> Result<Redirect, StatusCode> {
    let (auth_url, csrf_token, nonce) = state
        .oidc_client
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".to_owned()))
        .add_scope(Scope::new("profile".to_owned()))
        .url();

    session
        .insert("csrf_token", csrf_token.secret().clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    session
        .insert("nonce", nonce.secret().clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(auth_url.as_str()))
}

async fn auth_callback(
    State(state): State<EvaltorState>,
    session: Session,
    Query(params): Query<AuthCallbackParams>,
) -> Result<Redirect, StatusCode> {
    let stored_csrf: String = session
        .get("csrf_token")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::BAD_REQUEST)?;

    if stored_csrf != params.state {
        return Err(StatusCode::BAD_REQUEST);
    }

    let stored_nonce: String = session
        .get("nonce")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::BAD_REQUEST)?;

    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let token_response = state
        .oidc_client
        .exchange_code(AuthorizationCode::new(params.code))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .request_async(&http_client)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let id_token = token_response
        .id_token()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let claims = id_token
        .claims(
            &state.oidc_client.id_token_verifier(),
            &Nonce::new(stored_nonce),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let google_sub = claims.subject().to_string();

    let email = claims
        .email()
        .map(|e| e.as_str().to_owned())
        .unwrap_or_default();

    let name = claims
        .name()
        .and_then(|n| n.get(None))
        .map(|n| n.as_str().to_owned())
        .unwrap_or_default();

    let existing_id = sqlx::query_scalar!(
        r#"SELECT id as "id: uuid::Uuid" FROM users WHERE google_sub = ?"#,
        google_sub
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user_id = if let Some(id) = existing_id {
        sqlx::query!(
            "UPDATE users SET email = ?, name = ? WHERE google_sub = ?",
            email,
            name,
            google_sub
        )
        .execute(&state.db_pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        id
    } else {
        let new_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO users (id, google_sub, email, name) VALUES (?, ?, ?, ?)",
            new_id,
            google_sub,
            email,
            name
        )
        .execute(&state.db_pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        new_id
    };

    session
        .insert("user_id", user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/"))
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage;

async fn login() -> Result<Html<String>, StatusCode> {
    LoginPage
        .render()
        .map(Html)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn logout(session: Session) -> Result<Redirect, StatusCode> {
    session
        .flush()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Redirect::to("/login"))
}

pub fn auth_router() -> Router<EvaltorState> {
    Router::new()
        .route("/login", get(login))
        .route("/auth/google", get(google_login))
        .route("/auth/callback", get(auth_callback))
        .route("/auth/logout", get(logout))
}
