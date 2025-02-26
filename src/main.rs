use anyhow::anyhow;
use askama_axum::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Form, Router,
};
use chrono::NaiveDate;
use core::panic;
use serde::Deserialize;
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

#[derive(Clone, Debug, Deserialize)]
struct Announcement {
    title: String,
    body: String,
    date: NaiveDate,
    author: String,
}

#[derive(Clone, Debug, Deserialize)]
struct AdminAnnouncement {
    id: i32,
    title: String,
    date: NaiveDate,
}

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

async fn home_inner(state: AppState) -> anyhow::Result<HomeTemplate> {
    // TODO: deal with this unwrap?
    let agenda = state.agenda.read().unwrap().clone();
    let announcements = {
        let conn = state.db_pool.get().await?;
        // TODO: deal with this unwrap?
        let conn = conn.lock().unwrap();
        let mut stmt =
            conn.prepare_cached("SELECT title, body, date, author FROM announcements")?;
        let announcements = stmt
            .query_map([], |row| {
                Ok(Announcement {
                    title: row.get(0)?,
                    body: row.get(1)?,
                    date: row.get(2)?,
                    author: row.get(3)?,
                })
            })?
            .map(|res| res.map_err(|e| anyhow!(e)))
            .collect::<Result<Vec<Announcement>, anyhow::Error>>()?;
        announcements
    };
    Ok(HomeTemplate {
        agenda,
        announcements,
    })
}

async fn home(State(state): State<AppState>) -> Result<HomeTemplate, AppError> {
    home_inner(state).await.map_err(AppError)
}

async fn admin_inner(state: AppState) -> anyhow::Result<AdminTemplate> {
    let announcements = {
        let conn = state.db_pool.get().await?;
        // TODO: deal with this unwrap?
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare_cached("SELECT id, title, date FROM announcements")?;
        let announcements = stmt
            .query_map([], |row| {
                Ok(AdminAnnouncement {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    date: row.get(2)?,
                })
            })?
            .map(|res| res.map_err(|e| anyhow!(e)))
            .collect::<Result<Vec<AdminAnnouncement>, anyhow::Error>>()?;
        announcements
    };
    Ok(AdminTemplate { announcements })
}

async fn admin(State(state): State<AppState>) -> Result<AdminTemplate, AppError> {
    admin_inner(state).await.map_err(AppError)
}

async fn get_new_announcement() -> Result<NewAnnouncementTemplate, AppError> {
    Ok(NewAnnouncementTemplate {})
}

async fn post_new_announcement_inner(
    state: AppState,
    announcement: Announcement,
) -> anyhow::Result<()> {
    let conn = state.db_pool.get().await?;
    // TODO: deal with this unwrap?
    let conn = conn.lock().unwrap();
    let mut stmt = conn.prepare_cached(
        "INSERT INTO announcements (title, body, date, author) VALUES ($1, $2, $3, $4)",
    )?;
    stmt.execute(rusqlite::params![
        announcement.title,
        announcement.body,
        announcement.date,
        announcement.author,
    ])?;

    Ok(())
}

async fn post_new_announcement(
    State(state): State<AppState>,
    Form(announcement): Form<Announcement>,
) -> Result<Redirect, AppError> {
    post_new_announcement_inner(state, announcement)
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
