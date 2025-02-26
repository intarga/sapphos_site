use askama_axum::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Form, Router,
};
use core::panic;
use std::sync::{Arc, RwLock};
use tower_http::{compression::CompressionLayer, services::ServeDir};
use tracing::error;

mod announcements;
use announcements::{AdminAnnouncement, Announcement};

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
    db_pool: deadpool_sqlite::Pool,
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    agenda: Agenda,
    announcements: Vec<Announcement>,
}

#[derive(Template)]
#[template(path = "admin.html")]
struct AdminTemplate {
    // agenda: Agenda,
    announcements: Vec<AdminAnnouncement>,
}

#[derive(Template)]
#[template(path = "new_announcement.html")]
struct NewAnnouncementTemplate {}

struct AppError(anyhow::Error);

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

async fn home(State(state): State<AppState>) -> Result<HomeTemplate, AppError> {
    // TODO: deal with this unwrap?
    let agenda = state.agenda.read().unwrap().clone();
    let announcements = announcements::select_announcements(state.db_pool)
        .await
        .map_err(AppError)?;

    Ok(HomeTemplate {
        agenda,
        announcements,
    })
}

async fn admin(State(state): State<AppState>) -> Result<AdminTemplate, AppError> {
    let announcements = announcements::select_admin_announcements(state.db_pool)
        .await
        .map_err(AppError)?;

    Ok(AdminTemplate { announcements })
}

async fn get_new_announcement() -> Result<NewAnnouncementTemplate, AppError> {
    Ok(NewAnnouncementTemplate {})
}

async fn post_new_announcement(
    State(state): State<AppState>,
    Form(announcement): Form<Announcement>,
) -> Result<Redirect, AppError> {
    announcements::insert_announcement(state.db_pool, announcement)
        .await
        .map_err(AppError)?;

    // TODO: should indicate success somehow?
    Ok(Redirect::to("/admin"))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let db_schema = std::fs::read_to_string("db/schema.sql").expect("Failed to read db/schema.sql");
    let deadpool_cfg = deadpool_sqlite::Config::new("db/db.sqlite3");
    let db_pool = deadpool_cfg
        .create_pool(deadpool_sqlite::Runtime::Tokio1)
        .expect("Failed to create DB pool");
    let conn = db_pool
        .get()
        .await
        .expect("Failed to get a DB connection from the pool");
    {
        let conn = conn.lock().expect("Failed to acquire lock on DB conn");
        conn.execute_batch(&db_schema)
            .expect("Failed to execute DB schema");
    }

    let agenda = match gcal::fetch_calendar().await {
        Ok(agenda) => agenda,
        Err(e) => {
            error!("Failed to initialise Agenda from GCal API: {}", e);
            panic!("Cannot start server without initial agenda state");
        }
    };

    let state = AppState {
        agenda: Arc::new(RwLock::new(agenda)),
        db_pool,
    };

    // Refresh the agenda in the background every 5 minutes
    let background_agenda = state.agenda.clone();
    tokio::task::spawn(gcal::refresh_agenda_at_interval(
        background_agenda,
        tokio::time::interval(tokio::time::Duration::from_secs(5 * 60)),
    ));

    let app = Router::new()
        .route("/", get(home))
        .route("/admin", get(admin))
        .route(
            "/new_announcement",
            get(get_new_announcement).post(post_new_announcement),
        )
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
