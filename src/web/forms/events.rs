use crate::{
    events::{self, Event},
    web::{forms::STYLESHEET_FORM, AdminNavTemplate, AppError, AppState, HeadTemplate, IdQuery},
};
use anyhow::anyhow;
use askama::Template;
use askama_web::WebTemplate;
use axum::{
    extract::{Query, State},
    response::Redirect,
    routing::get,
    Form, Router,
};
use chrono::ParseError;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Template, WebTemplate)]
#[template(path = "new_event.html")]
pub struct NewEventTemplate {
    head: HeadTemplate,
    nav: AdminNavTemplate,
}

impl NewEventTemplate {
    fn new() -> Self {
        Self {
            head: HeadTemplate {
                stylesheet: STYLESHEET_FORM,
            },
            nav: AdminNavTemplate {},
        }
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "edit_event.html")]
pub struct EditEventTemplate {
    head: HeadTemplate,
    nav: AdminNavTemplate,
    id: i64,
    event: Event,
}

impl EditEventTemplate {
    fn new(id: i64, event: Event) -> Self {
        Self {
            head: HeadTemplate {
                stylesheet: STYLESHEET_FORM,
            },
            nav: AdminNavTemplate {},
            id,
            event,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct FormEvent {
    title: String,
    location: String,
    description: String,
    start_date: String,
    end_date: String,
    start_time: String,
    end_time: String,
    host: String,
    host_email: String,
}

fn optional_field(input: String) -> Option<String> {
    if input.is_empty() {
        None
    } else {
        Some(input)
    }
}

fn parse_optional_field<T: FromStr>(input: String) -> Result<Option<T>, AppError>
where
    <T as FromStr>::Err: Send,
    <T as FromStr>::Err: Sync,
    <T as FromStr>::Err: std::error::Error,
    <T as FromStr>::Err: 'static,
{
    if input.is_empty() {
        Ok(None)
    } else {
        Ok(Some(
            input
                .parse()
                .map_err(|e: <T as FromStr>::Err| AppError(anyhow!(e)))?,
        ))
    }
}

impl TryInto<Event> for FormEvent {
    type Error = AppError;

    fn try_into(self) -> Result<Event, Self::Error> {
        Ok(Event {
            title: self.title,
            location: optional_field(self.location),
            description: optional_field(self.description),
            start_date: self
                .start_date
                .parse()
                .map_err(|e: ParseError| AppError(anyhow!(e)))?,
            end_date: parse_optional_field(self.end_date)?,
            start_time: parse_optional_field(self.start_time)?,
            end_time: parse_optional_field(self.end_time)?,
            host: optional_field(self.host),
            host_email: optional_field(self.host_email),
        })
    }
}

pub async fn get_new_event() -> Result<NewEventTemplate, AppError> {
    Ok(NewEventTemplate::new())
}

pub async fn post_new_event(
    State(state): State<AppState>,
    Form(event): Form<FormEvent>,
) -> Result<Redirect, AppError> {
    events::insert_event(state.db_pool, event.try_into()?)
        .await
        .map_err(AppError)?;

    // TODO: should indicate success somehow?
    Ok(Redirect::to("/admin"))
}

pub async fn get_edit_event(
    State(state): State<AppState>,
    query: Query<IdQuery>,
) -> Result<EditEventTemplate, AppError> {
    let event = events::select_event(state.db_pool, query.id)
        .await
        .map_err(AppError)?;

    Ok(EditEventTemplate::new(query.id, event))
}

pub async fn post_edit_event(
    State(state): State<AppState>,
    query: Query<IdQuery>,
    Form(event): Form<FormEvent>,
) -> Result<Redirect, AppError> {
    events::update_event(state.db_pool, query.id, event.try_into()?)
        .await
        .map_err(AppError)?;

    // TODO: should indicate success somehow?
    Ok(Redirect::to("/admin"))
}

pub async fn delete_event(
    State(state): State<AppState>,
    query: Query<IdQuery>,
) -> Result<Redirect, AppError> {
    events::delete_event(state.db_pool, query.id)
        .await
        .map_err(AppError)?;

    // TODO: should indicate success somehow?
    Ok(Redirect::to("/admin"))
}

pub fn router() -> axum::Router<AppState> {
    Router::new()
        .route("/new_event", get(get_new_event).post(post_new_event))
        .route("/edit_event", get(get_edit_event).post(post_edit_event))
        .route("/delete_event", get(delete_event))
}
