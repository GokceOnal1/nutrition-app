//! Liveness check. Unauthenticated by design: it's for process/uptime
//! monitoring, not for revealing anything about the app's data.

use axum::extract::State;

use crate::{error::AppError, AppState};

pub async fn health(State(state): State<AppState>) -> Result<&'static str, AppError> {
    sqlx::query("SELECT 1").execute(&state.pool).await?;
    Ok("ok")
}
