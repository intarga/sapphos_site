//! Implementation of tower-sessions' SessionStore for deadpool-sqlite as I couldn't find a maintained one

use async_trait::async_trait;
use deadpool_sqlite::Pool;
use rusqlite::{params, OptionalExtension};
use tower_sessions::{
    cookie::time::OffsetDateTime,
    session::{Id, Record},
    session_store, ExpiredDeletion, SessionStore,
};

#[derive(Clone, Debug)]
pub struct DeadpoolSessionStore {
    pool: Pool,
}

impl DeadpoolSessionStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

fn encode_err(e: impl std::error::Error) -> session_store::Error {
    session_store::Error::Encode(e.to_string())
}
fn decode_err(e: impl std::error::Error) -> session_store::Error {
    session_store::Error::Decode(e.to_string())
}
fn backend_err(e: impl std::error::Error) -> session_store::Error {
    session_store::Error::Backend(e.to_string())
}

#[async_trait]
impl SessionStore for DeadpoolSessionStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        let record_conn = record.clone();

        let conn = self.pool.get().await.map_err(backend_err)?;
        let id = conn
            .interact(move |conn| {
                let mut record = record_conn;
                let tx = conn.transaction().map_err(backend_err)?;

                {
                    let mut stmt_collision = tx
                        .prepare_cached("SELECT id FROM tower_sessions WHERE id = $1")
                        .map_err(backend_err)?;
                    while stmt_collision
                        .exists([record.id.to_string()])
                        .map_err(backend_err)?
                    {
                        record.id = Id::default()
                    }

                    let data = bincode::serialize(&record).map_err(encode_err)?;
                    let expiry = record.expiry_date.unix_timestamp();
                    let mut stmt_insert = tx
                        .prepare_cached(
                            "INSERT INTO tower_sessions (id, data, expiry) VALUES ($1, $2, $3)",
                        )
                        .map_err(backend_err)?;
                    stmt_insert
                        .execute(params![record.id.to_string(), data, expiry])
                        .map_err(backend_err)?;
                }

                tx.commit().map_err(backend_err)?;

                Ok::<_, session_store::Error>(record.id)
            })
            .await
            .map_err(backend_err)??;

        record.id = id;

        Ok(())
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        let id = record.id.clone();
        let data = bincode::serialize(record).map_err(encode_err)?;
        let expiry = record.expiry_date.unix_timestamp();

        let conn = self.pool.get().await.map_err(backend_err)?;
        conn.interact(move |conn| {
            let mut stmt = conn
                .prepare_cached("UPDATE tower_sessions SET data = $1, expiry = $2 WHERE id = $3")?;
            stmt.execute(params![data, expiry, id.to_string()])?;

            Ok::<_, rusqlite::Error>(())
        })
        .await
        .map_err(backend_err)?
        .map_err(backend_err)?;

        Ok(())
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        let id_string = session_id.to_string();

        let conn = self.pool.get().await.map_err(backend_err)?;
        let data = conn
            .interact(move |conn| {
                let mut stmt = conn.prepare_cached(
                    "SELECT data FROM tower_sessions WHERE id = $1 AND expiry > $2",
                )?;
                let now = OffsetDateTime::now_utc().unix_timestamp();
                let data = stmt
                    .query_row(params![id_string, now], |row| row.get::<_, Vec<u8>>(0))
                    .optional()?;

                Ok::<_, rusqlite::Error>(data)
            })
            .await
            .map_err(backend_err)?
            .map_err(backend_err)?;

        let record = data
            .map(|data| bincode::deserialize::<Record>(&data).map_err(decode_err))
            .transpose()?;

        Ok(record)
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        let id_string = session_id.to_string();

        let conn = self.pool.get().await.map_err(backend_err)?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached("DELETE FROM tower_sessions WHERE id = $1")?;
            stmt.execute([id_string])?;

            Ok::<_, rusqlite::Error>(())
        })
        .await
        .map_err(backend_err)?
        .map_err(backend_err)?;

        Ok(())
    }
}

#[async_trait]
impl ExpiredDeletion for DeadpoolSessionStore {
    async fn delete_expired(&self) -> session_store::Result<()> {
        let conn = self.pool.get().await.map_err(backend_err)?;
        conn.interact(|conn| {
            let mut stmt = conn.prepare_cached("DELETE FROM tower_sessions WHERE expiry < $1")?;
            stmt.execute([OffsetDateTime::now_utc().unix_timestamp()])?;

            Ok::<_, rusqlite::Error>(())
        })
        .await
        .map_err(backend_err)?
        .map_err(backend_err)?;

        Ok(())
    }
}
