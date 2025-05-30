use axum_login::AuthManagerLayerBuilder;
use core::panic;
use std::sync::{Arc, RwLock};
use tower_http::{compression::CompressionLayer, services::ServeDir};
use tower_sessions::SessionManagerLayer;
use tracing::error;

use sapphos_site::{
    auth, events,
    session_store::DeadpoolSessionStore,
    web::{self, AppState},
};

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

    let session_store = DeadpoolSessionStore::new(db_pool.clone());
    // NOTE: disabled since expiry is turned off, needs `deletion_task` feature on tower_sessions_core
    // background task to clean up expired sessions
    // tokio::task::spawn(
    //     session_store
    //         .clone()
    //         .continuously_delete_expired(tokio::time::Duration::from_secs(60)),
    // );

    let session_layer = SessionManagerLayer::new(session_store)
        // allow cookie on non-https sessions, as it doesn't contain any sensitive info
        .with_secure(false)
        // NOTE: leaving this off since we're a small trusted user group, and it would be nice to
        // not have to log in all the time. If you enable it you'll also need to enable the deletion task above
        //.with_expiry()
        .with_signed(tower_sessions::cookie::Key::generate());

    let auth_layer =
        AuthManagerLayerBuilder::new(auth::AuthBackend::new(db_pool.clone()), session_layer)
            .build();

    let agenda = match events::gcal::fetch_calendar().await {
        Ok(agenda) => agenda,
        Err(e) => {
            error!("Failed to initialise Agenda from GCal API: {}", e);
            panic!("Cannot start server without initial agenda state");
        }
    };

    let state = AppState {
        gcal_agenda: Arc::new(RwLock::new(agenda)),
        db_pool,
    };

    // Refresh the agenda in the background every 5 minutes
    let background_agenda = state.gcal_agenda.clone();
    tokio::task::spawn(events::gcal::refresh_agenda_at_interval(
        background_agenda,
        tokio::time::interval(tokio::time::Duration::from_secs(5 * 60)),
    ));

    let app = web::router()
        .with_state(state)
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(auth_layer)
        .layer(CompressionLayer::new());

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind port");
    axum::serve(listener, app)
        .await
        .expect("failed to serve app");
}
