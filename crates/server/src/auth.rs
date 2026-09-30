//! Single-user password auth: an argon2 hash checked against `APP_PASSWORD_HASH`,
//! backed by a `tower-sessions` cookie. There is no signup flow and no user table
//! (see `CLAUDE.md`: not a product, single user, no multi-tenancy).

use argon2::{
    password_hash::{phc::PasswordHash, PasswordVerifier},
    Argon2,
};
use axum::{
    extract::{FromRequestParts, State},
    http::request::Parts,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{templates::LoginTemplate, AppState};

/// Session key under which the "is logged in" flag is stored.
const AUTH_KEY: &str = "authenticated";

#[derive(Deserialize)]
pub struct LoginForm {
    password: String,
}

/// Checks `password` against an argon2 PHC hash. Returns `Ok(false)` for a
/// plain mismatch and `Err` only if `hash` itself is not a valid PHC string
/// (a misconfigured `APP_PASSWORD_HASH`).
fn verify_password(hash: &str, password: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub async fn login_page() -> LoginTemplate {
    LoginTemplate { error: None }
}

pub async fn login_submit(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<LoginForm>,
) -> Response {
    match verify_password(&state.config.app_password_hash, &form.password) {
        Ok(true) => {
            if let Err(err) = session.insert(AUTH_KEY, true).await {
                tracing::error!(%err, "failed to persist session after login");
                return LoginTemplate {
                    error: Some("Something went wrong. Please try again.".to_string()),
                }
                .into_response();
            }
            Redirect::to("/").into_response()
        }
        Ok(false) => LoginTemplate {
            error: Some("Incorrect password.".to_string()),
        }
        .into_response(),
        Err(err) => {
            tracing::error!(%err, "APP_PASSWORD_HASH is not a valid argon2 PHC hash");
            LoginTemplate {
                error: Some("Server is misconfigured.".to_string()),
            }
            .into_response()
        }
    }
}

pub async fn logout(session: Session) -> Redirect {
    if let Err(err) = session.flush().await {
        tracing::error!(%err, "failed to clear session on logout");
    }
    Redirect::to("/login")
}

/// Extractor that guards a route behind the session's `authenticated` flag,
/// redirecting to `/login` otherwise. Use as a handler argument:
/// `async fn page(_auth: RequireAuth, ...)`.
pub struct RequireAuth;

impl<S> FromRequestParts<S> for RequireAuth
where
    S: Send + Sync,
{
    type Rejection = Redirect;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| Redirect::to("/login"))?;
        let authenticated: bool = session.get(AUTH_KEY).await.unwrap_or(None).unwrap_or(false);
        if authenticated {
            Ok(RequireAuth)
        } else {
            Err(Redirect::to("/login"))
        }
    }
}
