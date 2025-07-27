use crate::{
    events::{self, PageEvent},
    web::{AppError, AppState, HeadTemplate, HomeNavTemplate, NavTheme},
};
use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Path, State};

const STYLESHEET_HOME: &str = "home.css?v=1.3";

#[derive(Template, WebTemplate)]
#[template(path = "event.html")]
pub struct EventTemplate {
    head: HeadTemplate,
    nav: HomeNavTemplate,
    event: PageEvent,
    //agenda: Agenda,
    //announcements: Vec<Announcement>,
}

impl EventTemplate {
    fn new(event: PageEvent) -> Self {
        Self {
            head: HeadTemplate {
                stylesheet: STYLESHEET_HOME,
            },
            nav: HomeNavTemplate {
                nav_theme: NavTheme::Dark,
            },
            event,
        }
    }
}

pub async fn event(
    State(state): State<AppState>,
    Path(event_id): Path<i64>,
) -> Result<EventTemplate, AppError> {
    let event = events::select_event(state.db_pool.clone(), event_id)
        .await
        .map_err(AppError)?
        .into();

    Ok(EventTemplate::new(event))
}
