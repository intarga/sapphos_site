use crate::{
    announcements::{self, Announcement},
    events::{self, Agenda},
    web::{AppError, AppState, HeadTemplate, HomeNavTemplate, NavTheme},
};
use askama::Template;
use askama_web::WebTemplate;
use axum::extract::State;

const STYLESHEET_HOME: &str = "home.css?v=1.3";

#[derive(Template, WebTemplate)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    head: HeadTemplate,
    nav: HomeNavTemplate,
    agenda: Agenda,
    announcements: Vec<Announcement>,
}

impl HomeTemplate {
    fn new(agenda: Agenda, announcements: Vec<Announcement>) -> Self {
        Self {
            head: HeadTemplate {
                stylesheet: STYLESHEET_HOME,
            },
            nav: HomeNavTemplate {
                nav_theme: NavTheme::Dark,
            },
            agenda,
            announcements,
        }
    }
}

pub async fn home(State(state): State<AppState>) -> Result<HomeTemplate, AppError> {
    // TODO: deal with this unwrap?
    let gcal_agenda = state.gcal_agenda.read().unwrap().clone();
    let agenda = events::make_agenda(state.db_pool.clone(), gcal_agenda)
        .await
        .map_err(AppError)?;
    let announcements = announcements::select_announcements(state.db_pool)
        .await
        .map_err(AppError)?;

    Ok(HomeTemplate::new(agenda, announcements))
}
