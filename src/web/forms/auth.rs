use crate::{
    auth::{self, AuthSession, Credentials},
    web::{forms::STYLESHEET_FORM, AppError, AppState, HeadTemplate, NextQuery},
};
use askama::Template;
use askama_web::WebTemplate;
use axum::{extract::Query, response::Redirect, routing::get, Form, Router};

#[derive(Template, WebTemplate)]
#[template(path = "login.html")]
pub struct LoginTemplate {
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

pub async fn get_login(Query(NextQuery { next }): Query<NextQuery>) -> LoginTemplate {
    LoginTemplate::new(next)
}

pub async fn post_login(
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

pub async fn logout(auth_session: AuthSession) -> Result<Redirect, AppError> {
    auth::logout(auth_session).await.map_err(AppError)?;

    Ok(Redirect::to("/login?next=/admin"))
}

pub fn router() -> axum::Router<AppState> {
    Router::new()
        .route("/login", get(get_login).post(post_login))
        .route("/logout", get(logout))
}
