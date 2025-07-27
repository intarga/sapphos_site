use crate::{
    signal::{self, Chat},
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

#[derive(Template, WebTemplate)]
#[template(path = "new_chat.html")]
pub struct NewChatTemplate {
    head: HeadTemplate,
    nav: AdminNavTemplate,
}

impl NewChatTemplate {
    fn new() -> Self {
        Self {
            head: HeadTemplate {
                stylesheet: STYLESHEET_FORM,
                title: Some("Create a Chat".to_string()),
                description: None,
            },
            nav: AdminNavTemplate {},
        }
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "edit_chat.html")]
pub struct EditChatTemplate {
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
                title: Some("Edit a Chat".to_string()),
                description: None,
            },
            nav: AdminNavTemplate {},
            id,
            chat,
        }
    }
}

pub async fn get_new_chat() -> Result<NewChatTemplate, AppError> {
    Ok(NewChatTemplate::new())
}

pub async fn post_new_chat(
    State(state): State<AppState>,
    Form(chat): Form<Chat>,
) -> Result<Redirect, AppError> {
    signal::insert_chat(state.db_pool, chat)
        .await
        .map_err(AppError)?;

    // TODO: should indicate success somehow?
    Ok(Redirect::to("/admin"))
}

pub async fn get_edit_chat(
    State(state): State<AppState>,
    query: Query<IdQuery>,
) -> Result<EditChatTemplate, AppError> {
    let chat = signal::select_chat(state.db_pool, query.id)
        .await
        .map_err(AppError)?;

    Ok(EditChatTemplate::new(query.id, chat))
}

pub async fn post_edit_chat(
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

pub async fn delete_chat(
    State(state): State<AppState>,
    query: Query<IdQuery>,
) -> Result<Redirect, AppError> {
    signal::delete_chat(state.db_pool, query.id)
        .await
        .map_err(AppError)?;

    // TODO: should indicate success somehow?
    Ok(Redirect::to("/admin"))
}

pub fn router() -> axum::Router<AppState> {
    Router::new()
        .route("/new_chat", get(get_new_chat).post(post_new_chat))
        .route("/edit_chat", get(get_edit_chat).post(post_edit_chat))
        .route("/delete_chat", get(delete_chat))
}
