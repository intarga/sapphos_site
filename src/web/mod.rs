use crate::auth;
use askama::Template;
use askama_web::WebTemplate;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use axum_login::login_required;
use serde::Deserialize;

pub mod admin;
pub mod event;
pub mod forms;
pub mod home;
pub mod signal;

#[derive(Clone, Debug)]
pub struct AppState {
    pub db_pool: deadpool_sqlite::Pool,
}

pub struct AppError(anyhow::Error);

// TODO: make this a nice template
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

#[derive(Debug, Deserialize)]
pub struct IdQuery {
    id: i64,
}

#[derive(Debug, Deserialize)]
pub struct NextQuery {
    next: Option<String>,
}

enum NavTheme {
    Light,
    Dark,
}

impl std::fmt::Display for NavTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Light => write!(f, "light"),
            Self::Dark => write!(f, "dark"),
        }
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "partials/head.html")]
struct HeadTemplate {
    stylesheet: &'static str,
    title: Option<String>,
    description: Option<String>,
}

#[derive(Template, WebTemplate)]
#[template(path = "partials/home-nav.html")]
struct HomeNavTemplate {
    nav_theme: NavTheme,
}

#[derive(Template, WebTemplate)]
#[template(path = "partials/admin-nav.html")]
struct AdminNavTemplate {}

pub fn router() -> axum::Router<AppState> {
    Router::new()
        .route("/admin", get(admin::admin))
        .merge(forms::announcements::router())
        .merge(forms::events::router())
        .merge(forms::chats::router())
        .route_layer(login_required!(auth::AuthBackend, login_url = "/login"))
        .route("/", get(home::home))
        .route("/events/{event}", get(event::event))
        .merge(forms::auth::router())
        .route("/signal", get(signal::signal))
}
