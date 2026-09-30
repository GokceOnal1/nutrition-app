//! Askama template structs. Markup lives in `templates/`; this module only
//! holds the data each template needs and derives its response impls.

use askama::Template;
use askama_web::WebTemplate;

#[derive(Template, WebTemplate)]
#[template(path = "pages/login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
}

#[derive(Template, WebTemplate)]
#[template(path = "pages/home.html")]
pub struct HomeTemplate;
