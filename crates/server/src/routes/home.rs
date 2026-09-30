//! Placeholder authenticated landing page. Replaced by the real dashboard in
//! a later milestone; for now it only proves the login/session wiring works.

use crate::{auth::RequireAuth, templates::HomeTemplate};

pub async fn home(_auth: RequireAuth) -> HomeTemplate {
    HomeTemplate
}
