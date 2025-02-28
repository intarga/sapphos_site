use anyhow::anyhow;
use async_trait::async_trait;
use axum_login::{AuthUser, AuthnBackend, UserId};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    id: i64,
    pub username: String,
    password_hash: String,
}

// custom debug impl to avoid leaking password
impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("username", &self.username)
            .field("password", &"[redacted]")
            .finish()
    }
}

impl AuthUser for User {
    type Id = i64;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes() // We use the password hash as the auth
                                      // hash--what this means
                                      // is when the user changes their password the
                                      // auth session becomes invalid.
    }
}

// This allows us to extract the authentication fields from forms. We use this
// to authenticate requests with the backend.
#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    // page to redirect the user in after login
    pub next: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthBackend {
    db_pool: deadpool_sqlite::Pool,
}

impl AuthBackend {
    pub fn new(db: deadpool_sqlite::Pool) -> Self {
        Self { db_pool: db }
    }
}

// needed as anyhow::Error does not implement std::error::Error, so can't be directly used in
// the AuthnBackend impl
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Pool(#[from] deadpool_sqlite::PoolError),
    // deadpool_sqlite::InteractError is not Sync...
    #[error("{0}")]
    Interact(String),
    #[error(transparent)]
    Db(#[from] rusqlite::Error),
    #[error(transparent)]
    TaskJoin(#[from] tokio::task::JoinError),
}

#[async_trait]
impl AuthnBackend for AuthBackend {
    type User = User;
    type Credentials = Credentials;
    type Error = Error;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let conn = self.db_pool.get().await?;
        let user = conn
            .interact(|conn| {
                let mut stmt = conn.prepare_cached(
                    "SELECT id, username, password_hash FROM users WHERE username = $1",
                )?;
                let user = stmt
                    .query_row([creds.username], |row| {
                        Ok(User {
                            id: row.get(0)?,
                            username: row.get(1)?,
                            password_hash: row.get(2)?,
                        })
                    })
                    .optional()?;
                Ok::<_, rusqlite::Error>(user)
            })
            .await
            .map_err(|e| Error::Interact(e.to_string()))??;

        // Verifying the password is blocking and potentially slow, so we'll do so via
        // `spawn_blocking`.
        tokio::task::spawn_blocking(|| {
            // We're using password-based authentication--this works by comparing our form
            // input with an argon2 password hash.
            Ok(user.filter(|user| {
                password_auth::verify_password(creds.password, &user.password_hash).is_ok()
            }))
        })
        .await?
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        let id = *user_id;
        let conn = self.db_pool.get().await?;
        let user = conn
            .interact(move |conn| {
                let mut stmt = conn.prepare_cached(
                    "SELECT id, username, password_hash FROM users WHERE id = $1",
                )?;
                let user = stmt
                    .query_row([id], |row| {
                        Ok(User {
                            id: row.get(0)?,
                            username: row.get(1)?,
                            password_hash: row.get(2)?,
                        })
                    })
                    .optional()?;
                Ok::<_, rusqlite::Error>(user)
            })
            .await
            .map_err(|e| Error::Interact(e.to_string()))??;

        Ok(user)
    }
}

// We use a type alias for convenience.
//
// Note that we've supplied our concrete backend here.
pub type AuthSession = axum_login::AuthSession<AuthBackend>;

// Helper functions for neater error handling in the router
pub async fn login(mut auth_session: AuthSession, creds: &Credentials) -> anyhow::Result<()> {
    let user = match auth_session.authenticate(creds.clone()).await? {
        Some(user) => user,
        // TODO: make this a flash message instead of an error page
        None => return Err(anyhow!("Invalid credentials")),
    };

    auth_session.login(&user).await?;

    Ok(())
}

pub async fn logout(mut auth_session: AuthSession) -> anyhow::Result<()> {
    auth_session.logout().await?;

    Ok(())
}
