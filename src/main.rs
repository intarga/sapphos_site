use askama_axum::Template;
use axum::{extract::State, routing::get, Router};
use core::panic;
use std::sync::{Arc, RwLock};
use tower_http::{compression::CompressionLayer, services::ServeDir};
use tracing::{error, info};

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
    // TODO: should this contain the rendered template instead?
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
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let agenda = match gcal::fetch_calendar().await {
        Ok(agenda) => agenda,
        Err(e) => {
            error!("Failed to initialise Agenda from GCal API: {}", e);
            panic!("Cannot start server without initial state");
        }
    };

    let state = AppState {
        agenda: Arc::new(RwLock::new(agenda)),
    };

    // Refresh the agenda in the background every 5 minutes
    let background_agenda = state.agenda.clone();
    tokio::task::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5 * 60));

        'refresh: loop {
            interval.tick().await;
            let new_agenda = match gcal::fetch_calendar().await {
                Ok(agenda) => agenda,
                Err(e) => {
                    error!("Failed to refresh Agenda from GCal API: {}", e);
                    continue 'refresh;
                }
            };
            let mut agenda = match background_agenda.write() {
                Ok(lock) => lock,
                Err(e) => {
                    error!("Failed to acquire lock on background state: {}", e);
                    continue 'refresh;
                }
            };
            *agenda = new_agenda;
            info!("Successfully refreshed agenda");
        }
    });

    let app = Router::new()
        .route("/", get(home))
        .with_state(state)
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(CompressionLayer::new());

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind port");
    axum::serve(listener, app)
        .await
        .expect("failed to serve app");
}
