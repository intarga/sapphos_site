use crate::{
    signal::{self, Chat},
    web::{AppError, AppState, HeadTemplate, HomeNavTemplate, NavTheme},
};
use askama::Template;
use askama_web::WebTemplate;
use axum::extract::State;

const STYLESHEET_SIGNAL: &str = "signal.css?v=1.3";

#[derive(Template, WebTemplate)]
#[template(path = "signal.html")]
pub struct SignalTemplate {
    head: HeadTemplate,
    nav: HomeNavTemplate,
    chats: Vec<Chat>,
}

impl SignalTemplate {
    fn new(subchats: Vec<Chat>) -> Self {
        Self {
            head: HeadTemplate {
                stylesheet: STYLESHEET_SIGNAL,
            },
            nav: HomeNavTemplate {
                nav_theme: NavTheme::Light,
            },
            chats: subchats,
        }
    }
}

pub async fn signal(State(state): State<AppState>) -> Result<SignalTemplate, AppError> {
    let chats = signal::select_chats(state.db_pool)
        .await
        .map_err(AppError)?;

    Ok(SignalTemplate::new(chats))
}
