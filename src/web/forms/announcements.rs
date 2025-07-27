use crate::{
    announcements::{self, Announcement},
    auth::AuthSession,
    web::{forms::STYLESHEET_FORM, AdminNavTemplate, AppError, AppState, HeadTemplate, IdQuery},
};
use askama::Template;
use askama_web::WebTemplate;
use axum::{
    extract::{Query, State},
    response::Redirect,
    routing::get,
    Form, Router,
};
use chrono::NaiveDate;
use chrono_tz::Europe::Oslo;

#[derive(Template, WebTemplate)]
#[template(path = "new_announcement.html")]
pub struct NewAnnouncementTemplate {
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
                title: Some("Create an Announcement".to_string()),
                description: None,
            },
            nav: AdminNavTemplate {},
            author,
            date,
        }
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "edit_announcement.html")]
pub struct EditAnnouncementTemplate {
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
                title: Some("Edit an Announcement".to_string()),
                description: None,
            },
            nav: AdminNavTemplate {},
            id,
            announcement,
        }
    }
}

pub async fn get_new_announcement(
    auth_session: AuthSession,
) -> Result<NewAnnouncementTemplate, AppError> {
    let author = auth_session
        .user
        .map(|user| user.username)
        .unwrap_or_default();
    let date = chrono::Utc::now().with_timezone(&Oslo).naive_local().date();

    Ok(NewAnnouncementTemplate::new(author, date))
}

pub async fn post_new_announcement(
    State(state): State<AppState>,
    Form(announcement): Form<Announcement>,
) -> Result<Redirect, AppError> {
    announcements::insert_announcement(state.db_pool, announcement)
        .await
        .map_err(AppError)?;

    // TODO: should indicate success somehow?
    Ok(Redirect::to("/admin"))
}

pub async fn get_edit_announcement(
    State(state): State<AppState>,
    query: Query<IdQuery>,
) -> Result<EditAnnouncementTemplate, AppError> {
    let announcement = announcements::select_announcement(state.db_pool, query.id)
        .await
        .map_err(AppError)?;

    Ok(EditAnnouncementTemplate::new(query.id, announcement))
}

pub async fn post_edit_announcement(
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

pub async fn delete_announcement(
    State(state): State<AppState>,
    query: Query<IdQuery>,
) -> Result<Redirect, AppError> {
    announcements::delete_announcement(state.db_pool, query.id)
        .await
        .map_err(AppError)?;

    // TODO: should indicate success somehow?
    Ok(Redirect::to("/admin"))
}

pub fn router() -> axum::Router<AppState> {
    Router::new()
        .route(
            "/new_announcement",
            get(get_new_announcement).post(post_new_announcement),
        )
        .route(
            "/edit_announcement",
            get(get_edit_announcement).post(post_edit_announcement),
        )
        .route("/delete_announcement", get(delete_announcement))
}
