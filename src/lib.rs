/// Utils for dealing with announcements
mod announcements;

/// Utils for dealing with events
pub mod events;

/// Utils for dealing with signal chats
pub mod signal;

/// Session-tracking cookies (needed for login) backed by our sqlite db
pub mod session_store;

/// Plumbing to make authentication work
pub mod auth;

/// Routes and templates
pub mod web;
