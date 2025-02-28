use crate::{
    announcements::{self, AdminAnnouncement, Announcement},
    auth::{self, AuthSession, Credentials},
    Agenda, AppState,
};
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
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct IdQuery {
    id: i32,
}

#[derive(Debug, Deserialize)]
struct NextQuery {
    next: Option<String>,
}

#[derive(Template, WebTemplate)]
#[template(path = "home.html")]
struct HomeTemplate {
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
    // agenda: Agenda,
    announcements: Vec<AdminAnnouncement>,
}

#[derive(Template, WebTemplate)]
#[template(path = "new_announcement.html")]
struct NewAnnouncementTemplate {}

#[derive(Template, WebTemplate)]
#[template(path = "edit_announcement.html")]
struct EditAnnouncementTemplate {
    id: i32,
    announcement: Announcement,
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

async fn get_edit_announcement(
    State(state): State<AppState>,
    query: Query<IdQuery>,
) -> Result<EditAnnouncementTemplate, AppError> {
    let announcement = announcements::select_announcement(state.db_pool, query.id)
        .await
        .map_err(AppError)?;

    Ok(EditAnnouncementTemplate {
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

    Ok(Redirect::to("/login"))
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
        .route_layer(login_required!(auth::AuthBackend, login_url = "/login"))
        .route("/", get(home))
        .route("/login", get(get_login).post(post_login))
        .route("/logout", get(logout))
}
