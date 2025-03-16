use crate::{
    announcements::{self, AdminAnnouncement, Announcement},
    auth::{self, AuthSession, Credentials},
    events::{self, AdminEvent, Agenda, Event},
    AppState,
};
use anyhow::anyhow;
use askama::Template;
use askama_web::WebTemplate;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Form, Router,
};
use axum_login::login_required;
use chrono::ParseError;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
struct IdQuery {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct NextQuery {
    next: Option<String>,
}

#[derive(Template, WebTemplate)]
#[template(path = "partials/home-nav.html")]
struct HomeNavTemplate {}

#[derive(Template, WebTemplate)]
#[template(path = "partials/admin-nav.html")]
struct AdminNavTemplate {}

#[derive(Template, WebTemplate)]
#[template(path = "home.html")]
struct HomeTemplate {
    nav: HomeNavTemplate,
    agenda: Agenda,
    announcements: Vec<Announcement>,
}

#[derive(Template, WebTemplate)]
#[template(path = "login.html")]
struct LoginTemplate {
    next: Option<String>,
}

#[derive(Template, WebTemplate)]
#[template(path = "admin.html")]
struct AdminTemplate {
    nav: AdminNavTemplate,
    announcements: Vec<AdminAnnouncement>,
    events: Vec<AdminEvent>,
}

#[derive(Template, WebTemplate)]
#[template(path = "new_announcement.html")]
struct NewAnnouncementTemplate {
    nav: AdminNavTemplate,
}

#[derive(Template, WebTemplate)]
#[template(path = "edit_announcement.html")]
struct EditAnnouncementTemplate {
    nav: AdminNavTemplate,
    id: i64,
    announcement: Announcement,
}

#[derive(Template, WebTemplate)]
#[template(path = "new_event.html")]
struct NewEventTemplate {
    nav: AdminNavTemplate,
}

#[derive(Template, WebTemplate)]
#[template(path = "edit_event.html")]
struct EditEventTemplate {
    nav: AdminNavTemplate,
    id: i64,
    event: Event,
}

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

#[derive(Clone, Debug, Deserialize)]
struct FormEvent {
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
    if input == "" {
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
    if input == "" {
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

async fn home(State(state): State<AppState>) -> Result<HomeTemplate, AppError> {
    // TODO: deal with this unwrap?
    let _gcal_agenda = state.gcal_agenda.read().unwrap().clone();
    let agenda = events::make_agenda(state.db_pool.clone())
        .await
        .map_err(AppError)?;
    let announcements = announcements::select_announcements(state.db_pool)
        .await
        .map_err(AppError)?;

    Ok(HomeTemplate {
        nav: HomeNavTemplate {},
        agenda,
        announcements,
    })
}

async fn admin(State(state): State<AppState>) -> Result<AdminTemplate, AppError> {
    let announcements = announcements::select_admin_announcements(state.db_pool.clone())
        .await
        .map_err(AppError)?;

    let events = events::select_admin_events(state.db_pool)
        .await
        .map_err(AppError)?;

    Ok(AdminTemplate {
        nav: AdminNavTemplate {},
        announcements,
        events,
    })
}

async fn get_new_announcement() -> Result<NewAnnouncementTemplate, AppError> {
    Ok(NewAnnouncementTemplate {
        nav: AdminNavTemplate {},
    })
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

async fn get_edit_announcement(
    State(state): State<AppState>,
    query: Query<IdQuery>,
) -> Result<EditAnnouncementTemplate, AppError> {
    let announcement = announcements::select_announcement(state.db_pool, query.id)
        .await
        .map_err(AppError)?;

    Ok(EditAnnouncementTemplate {
        nav: AdminNavTemplate {},
        id: query.id,
        announcement,
    })
}

async fn post_edit_announcement(
    State(state): State<AppState>,
    query: Query<IdQuery>,
    Form(announcement): Form<Announcement>,
) -> Result<Redirect, AppError> {
    announcements::update_announcement(state.db_pool, query.id, announcement)
        .await
        .map_err(AppError)?;

    // TODO: should indicate success somehow?
    Ok(Redirect::to("/admin"))
}

async fn delete_announcement(
    State(state): State<AppState>,
    query: Query<IdQuery>,
) -> Result<Redirect, AppError> {
    announcements::delete_announcement(state.db_pool, query.id)
        .await
        .map_err(AppError)?;

    // TODO: should indicate success somehow?
    Ok(Redirect::to("/admin"))
}

async fn get_new_event() -> Result<NewEventTemplate, AppError> {
    Ok(NewEventTemplate {
        nav: AdminNavTemplate {},
    })
}

async fn post_new_event(
    State(state): State<AppState>,
    Form(event): Form<FormEvent>,
) -> Result<Redirect, AppError> {
    println!("{:?}", event);
    events::insert_event(state.db_pool, event.try_into()?)
        .await
        .map_err(AppError)?;

    // TODO: should indicate success somehow?
    Ok(Redirect::to("/admin"))
}

async fn get_edit_event(
    State(state): State<AppState>,
    query: Query<IdQuery>,
) -> Result<EditEventTemplate, AppError> {
    let event = events::select_event(state.db_pool, query.id)
        .await
        .map_err(AppError)?;

    Ok(EditEventTemplate {
        nav: AdminNavTemplate {},
        id: query.id,
        event,
    })
}

async fn post_edit_event(
    State(state): State<AppState>,
    query: Query<IdQuery>,
    Form(event): Form<Event>,
) -> Result<Redirect, AppError> {
    events::update_event(state.db_pool, query.id, event)
        .await
        .map_err(AppError)?;

    // TODO: should indicate success somehow?
    Ok(Redirect::to("/admin"))
}

async fn delete_event(
    State(state): State<AppState>,
    query: Query<IdQuery>,
) -> Result<Redirect, AppError> {
    events::delete_event(state.db_pool, query.id)
        .await
        .map_err(AppError)?;

    // TODO: should indicate success somehow?
    Ok(Redirect::to("/admin"))
}

async fn get_login(Query(NextQuery { next }): Query<NextQuery>) -> LoginTemplate {
    LoginTemplate { next }
}

async fn post_login(
    auth_session: AuthSession,
    Form(creds): Form<Credentials>,
) -> Result<Redirect, AppError> {
    auth::login(auth_session, &creds).await.map_err(AppError)?;

    let redirect = match creds.next {
        Some(next) => Redirect::to(&next),
        None => Redirect::to("/"),
    };

    Ok(redirect)
}

async fn logout(auth_session: AuthSession) -> Result<Redirect, AppError> {
    auth::logout(auth_session).await.map_err(AppError)?;

    Ok(Redirect::to("/login?next=/admin"))
}

pub fn router() -> axum::Router<AppState> {
    Router::new()
        .route("/admin", get(admin))
        .route(
            "/new_announcement",
            get(get_new_announcement).post(post_new_announcement),
        )
        .route(
            "/edit_announcement",
            get(get_edit_announcement).post(post_edit_announcement),
        )
        .route("/delete_announcement", get(delete_announcement))
        .route("/new_event", get(get_new_event).post(post_new_event))
        .route("/edit_event", get(get_edit_event).post(post_edit_event))
        .route("/delete_event", get(delete_event))
        .route_layer(login_required!(auth::AuthBackend, login_url = "/login"))
        .route("/", get(home))
        .route("/login", get(get_login).post(post_login))
        .route("/logout", get(logout))
}
