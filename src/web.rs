use crate::{
    announcements::{self, AdminAnnouncement, Announcement},
    auth::{self, AuthSession, Credentials},
    events::{self, AdminEvent, Agenda, Event},
    signal::{self, AdminChat, Chat},
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
use chrono::{NaiveDate, ParseError};
use chrono_tz::Europe::Oslo;
use serde::Deserialize;
use std::str::FromStr;

const STYLESHEET_HOME: &str = "home.css?v=1.3";
const STYLESHEET_SIGNAL: &str = "signal.css?v=1.3";
const STYLESHEET_ADMIN: &str = "admin.css?v=1.3";
const STYLESHEET_FORM: &str = "form.css?v=1.2";

#[derive(Debug, Deserialize)]
struct IdQuery {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct NextQuery {
    next: Option<String>,
}

enum NavTheme {
    Light,
    Dark,
}

impl std::fmt::Display for NavTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Light => write!(f, "light"),
            Self::Dark => write!(f, "dark"),
        }
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "partials/head.html")]
struct HeadTemplate {
    stylesheet: &'static str,
}

#[derive(Template, WebTemplate)]
#[template(path = "partials/home-nav.html")]
struct HomeNavTemplate {
    nav_theme: NavTheme,
}

#[derive(Template, WebTemplate)]
#[template(path = "partials/admin-nav.html")]
struct AdminNavTemplate {}

#[derive(Template, WebTemplate)]
#[template(path = "home.html")]
struct HomeTemplate {
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

#[derive(Template, WebTemplate)]
#[template(path = "signal.html")]
struct SignalTemplate {
    head: HeadTemplate,
    nav: HomeNavTemplate,
    chats: Vec<Chat>,
}

impl SignalTemplate {
    fn new(subchats: Vec<Chat>) -> Self {
        Self {
            head: HeadTemplate {
                stylesheet: STYLESHEET_SIGNAL,
            },
            nav: HomeNavTemplate {
                nav_theme: NavTheme::Light,
            },
            chats: subchats,
        }
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "login.html")]
struct LoginTemplate {
    head: HeadTemplate,
    next: Option<String>,
}

impl LoginTemplate {
    fn new(next: Option<String>) -> Self {
        Self {
            head: HeadTemplate {
                stylesheet: STYLESHEET_FORM,
            },
            next,
        }
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "admin.html")]
struct AdminTemplate {
    head: HeadTemplate,
    nav: AdminNavTemplate,
    announcements: Vec<AdminAnnouncement>,
    events: Vec<AdminEvent>,
    chats: Vec<AdminChat>,
}

impl AdminTemplate {
    fn new(
        announcements: Vec<AdminAnnouncement>,
        events: Vec<AdminEvent>,
        chats: Vec<AdminChat>,
    ) -> Self {
        Self {
            head: HeadTemplate {
                stylesheet: STYLESHEET_ADMIN,
            },
            nav: AdminNavTemplate {},
            announcements,
            events,
            chats,
        }
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "new_announcement.html")]
struct NewAnnouncementTemplate {
    head: HeadTemplate,
    nav: AdminNavTemplate,
    author: String,
    date: NaiveDate,
}

impl NewAnnouncementTemplate {
    fn new(author: String, date: NaiveDate) -> Self {
        Self {
            head: HeadTemplate {
                stylesheet: STYLESHEET_FORM,
            },
            nav: AdminNavTemplate {},
            author,
            date,
        }
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "edit_announcement.html")]
struct EditAnnouncementTemplate {
    head: HeadTemplate,
    nav: AdminNavTemplate,
    id: i64,
    announcement: Announcement,
}

impl EditAnnouncementTemplate {
    fn new(id: i64, announcement: Announcement) -> Self {
        Self {
            head: HeadTemplate {
                stylesheet: STYLESHEET_FORM,
            },
            nav: AdminNavTemplate {},
            id,
            announcement,
        }
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "new_chat.html")]
struct NewChatTemplate {
    head: HeadTemplate,
    nav: AdminNavTemplate,
}

impl NewChatTemplate {
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
#[template(path = "edit_chat.html")]
struct EditChatTemplate {
    head: HeadTemplate,
    nav: AdminNavTemplate,
    id: i64,
    chat: Chat,
}

impl EditChatTemplate {
    fn new(id: i64, chat: Chat) -> Self {
        Self {
            head: HeadTemplate {
                stylesheet: STYLESHEET_FORM,
            },
            nav: AdminNavTemplate {},
            id,
            chat,
        }
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "new_event.html")]
struct NewEventTemplate {
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
struct EditEventTemplate {
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

async fn home(State(state): State<AppState>) -> Result<HomeTemplate, AppError> {
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

async fn signal(State(state): State<AppState>) -> Result<SignalTemplate, AppError> {
    let chats = signal::select_chats(state.db_pool)
        .await
        .map_err(AppError)?;

    Ok(SignalTemplate::new(chats))
}

async fn admin(State(state): State<AppState>) -> Result<AdminTemplate, AppError> {
    let announcements = announcements::select_admin_announcements(state.db_pool.clone())
        .await
        .map_err(AppError)?;

    let events = events::select_admin_events(state.db_pool.clone())
        .await
        .map_err(AppError)?;

    let chats = signal::select_admin_chats(state.db_pool)
        .await
        .map_err(AppError)?;

    Ok(AdminTemplate::new(announcements, events, chats))
}

async fn get_new_announcement(
    auth_session: AuthSession,
) -> Result<NewAnnouncementTemplate, AppError> {
    let author = auth_session
        .user
        .map(|user| user.username)
        .unwrap_or_default();
    let date = chrono::Utc::now().with_timezone(&Oslo).naive_local().date();

    Ok(NewAnnouncementTemplate::new(author, date))
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

    Ok(EditAnnouncementTemplate::new(query.id, announcement))
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
    Ok(NewEventTemplate::new())
}

async fn post_new_event(
    State(state): State<AppState>,
    Form(event): Form<FormEvent>,
) -> Result<Redirect, AppError> {
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

    Ok(EditEventTemplate::new(query.id, event))
}

async fn post_edit_event(
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

async fn get_new_chat() -> Result<NewChatTemplate, AppError> {
    Ok(NewChatTemplate::new())
}

async fn post_new_chat(
    State(state): State<AppState>,
    Form(chat): Form<Chat>,
) -> Result<Redirect, AppError> {
    signal::insert_chat(state.db_pool, chat)
        .await
        .map_err(AppError)?;

    // TODO: should indicate success somehow?
    Ok(Redirect::to("/admin"))
}

async fn get_edit_chat(
    State(state): State<AppState>,
    query: Query<IdQuery>,
) -> Result<EditChatTemplate, AppError> {
    let chat = signal::select_chat(state.db_pool, query.id)
        .await
        .map_err(AppError)?;

    Ok(EditChatTemplate::new(query.id, chat))
}

async fn post_edit_chat(
    State(state): State<AppState>,
    query: Query<IdQuery>,
    Form(chat): Form<Chat>,
) -> Result<Redirect, AppError> {
    signal::update_chat(state.db_pool, query.id, chat)
        .await
        .map_err(AppError)?;

    // TODO: should indicate success somehow?
    Ok(Redirect::to("/admin"))
}

async fn delete_chat(
    State(state): State<AppState>,
    query: Query<IdQuery>,
) -> Result<Redirect, AppError> {
    signal::delete_chat(state.db_pool, query.id)
        .await
        .map_err(AppError)?;

    // TODO: should indicate success somehow?
    Ok(Redirect::to("/admin"))
}

async fn get_login(Query(NextQuery { next }): Query<NextQuery>) -> LoginTemplate {
    LoginTemplate::new(next)
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
        .route("/new_chat", get(get_new_chat).post(post_new_chat))
        .route("/edit_chat", get(get_edit_chat).post(post_edit_chat))
        .route("/delete_chat", get(delete_chat))
        .route_layer(login_required!(auth::AuthBackend, login_url = "/login"))
        .route("/", get(home))
        .route("/login", get(get_login).post(post_login))
        .route("/logout", get(logout))
        .route("/signal", get(signal))
}
