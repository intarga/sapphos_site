use askama_axum::Template;
use axum::{extract::State, routing::get, Router};
use chrono::{DateTime, Utc};
use std::sync::{Arc, RwLock};
use tower_http::services::ServeDir;

mod gcal;

#[derive(Clone, Debug)]
pub struct Event {
    pub title: String,
    pub description: String,
    pub location: String,
    pub start_time: String,
    pub end_time: String,
    pub day: String,
}

#[derive(Clone, Debug)]
pub struct AgendaMonth {
    pub month: String,
    // the usize is an index used to decide how to colour the event divider
    pub events: Vec<(usize, Event)>,
}

pub type Agenda = Vec<AgendaMonth>;

#[derive(Clone, Debug)]
struct AppState {
    agenda: Arc<RwLock<Agenda>>,
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    agenda: Agenda,
}

async fn home(State(state): State<AppState>) -> HomeTemplate {
    let agenda = state.agenda.read().unwrap().clone();
    HomeTemplate { agenda }
}

#[tokio::main]
async fn main() {
    let state = AppState {
        agenda: Arc::new(RwLock::new(gcal::fetch_calendar().await)),
    };

    let app = Router::new()
        .route("/", get(home))
        .with_state(state)
        .nest_service("/assets", ServeDir::new("assets"));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
