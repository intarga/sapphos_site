use crate::{
    announcements::{self, AdminAnnouncement},
    events::{self, AdminEvent},
    signal::{self, AdminChat},
    web::{AdminNavTemplate, AppError, AppState, HeadTemplate},
};
use askama::Template;
use askama_web::WebTemplate;
use axum::extract::State;

const STYLESHEET_ADMIN: &str = "admin.css?v=1.3";

#[derive(Template, WebTemplate)]
#[template(path = "admin.html")]
pub struct AdminTemplate {
    head: HeadTemplate,
    nav: AdminNavTemplate,
    announcements: Vec<AdminAnnouncement>,
    events: Vec<AdminEvent>,
    chats: Vec<AdminChat>,
}

impl AdminTemplate {
    fn new(
        announcements: Vec<AdminAnnouncement>,
        events: Vec<AdminEvent>,
        chats: Vec<AdminChat>,
    ) -> Self {
        Self {
            head: HeadTemplate {
                stylesheet: STYLESHEET_ADMIN,
                title: Some("Admin Panel".to_string()),
                description: None,
            },
            nav: AdminNavTemplate {},
            announcements,
            events,
            chats,
        }
    }
}

pub async fn admin(State(state): State<AppState>) -> Result<AdminTemplate, AppError> {
    let announcements = announcements::select_admin_announcements(state.db_pool.clone())
        .await
        .map_err(AppError)?;

    let events = events::select_admin_events(state.db_pool.clone())
        .await
        .map_err(AppError)?;

    let chats = signal::select_admin_chats(state.db_pool)
        .await
        .map_err(AppError)?;

    Ok(AdminTemplate::new(announcements, events, chats))
}
