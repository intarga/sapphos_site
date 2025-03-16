use std::sync::{Arc, RwLock};

/// Utils for dealing with announcements
mod announcements;

/// Utils for dealing with events
pub mod events;
use events::Event;

/// Session-tracking cookies (needed for login) backed by our sqlite db
pub mod session_store;

/// Plumbing to make authentication work
pub mod auth;

/// Routes and templates
pub mod web;

#[derive(Clone, Debug)]
pub struct AppState {
    // TODO: should this contain the rendered template instead?
    pub gcal_agenda: Arc<RwLock<Vec<Event>>>,
    pub db_pool: deadpool_sqlite::Pool,
}
