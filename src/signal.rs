use anyhow::anyhow;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Chat {
    pub name: String,
    pub link: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AdminChat {
    pub id: i64,
    pub name: String,
    pub link: String,
}

pub async fn select_chat(db_pool: deadpool_sqlite::Pool, id: i64) -> anyhow::Result<Chat> {
    let conn = db_pool.get().await?;
    // TODO: deal with this unwrap?
    let conn = conn.lock().unwrap();
    let mut stmt = conn.prepare_cached("SELECT name, link FROM signal_chats WHERE id = $1")?;
    let chat = stmt.query_row([id], |row| {
        Ok(Chat {
            name: row.get(0)?,
            link: row.get(1)?,
        })
    })?;

    Ok(chat)
}

pub async fn select_chats(db_pool: deadpool_sqlite::Pool) -> anyhow::Result<Vec<Chat>> {
    let conn = db_pool.get().await?;
    // TODO: deal with this unwrap?
    let conn = conn.lock().unwrap();
    let mut stmt =
        conn.prepare_cached("SELECT id, name, link FROM signal_chats ORDER BY id ASC")?;
    let chats = stmt
        .query_map([], |row| {
            Ok(Chat {
                name: row.get(1)?,
                link: row.get(2)?,
            })
        })?
        .map(|res| res.map_err(|e| anyhow!(e)))
        .collect::<Result<Vec<Chat>, anyhow::Error>>()?;

    Ok(chats)
}

pub async fn select_admin_chats(db_pool: deadpool_sqlite::Pool) -> anyhow::Result<Vec<AdminChat>> {
    let conn = db_pool.get().await?;
    // TODO: deal with this unwrap?
    let conn = conn.lock().unwrap();
    let mut stmt =
        conn.prepare_cached("SELECT id, name, link FROM signal_chats ORDER BY id ASC")?;
    let chats = stmt
        .query_map([], |row| {
            Ok(AdminChat {
                id: row.get(0)?,
                name: row.get(1)?,
                link: row.get(2)?,
            })
        })?
        .map(|res| res.map_err(|e| anyhow!(e)))
        .collect::<Result<Vec<AdminChat>, anyhow::Error>>()?;

    Ok(chats)
}

pub async fn insert_chat(db_pool: deadpool_sqlite::Pool, chat: Chat) -> anyhow::Result<()> {
    let conn = db_pool.get().await?;
    // TODO: deal with this unwrap?
    let conn = conn.lock().unwrap();
    let mut stmt = conn.prepare_cached("INSERT INTO signal_chats (name, link) VALUES ($1, $2)")?;
    stmt.execute(rusqlite::params![chat.name, chat.link])?;

    Ok(())
}

pub async fn update_chat(
    db_pool: deadpool_sqlite::Pool,
    id: i64,
    chat: Chat,
) -> anyhow::Result<()> {
    let conn = db_pool.get().await?;
    // TODO: deal with this unwrap?
    let conn = conn.lock().unwrap();
    let mut stmt =
        conn.prepare_cached("UPDATE signal_chats SET name = $1, link = $2 WHERE id = $3")?;
    stmt.execute(rusqlite::params![chat.name, chat.link, id])?;

    Ok(())
}

pub async fn delete_chat(db_pool: deadpool_sqlite::Pool, id: i64) -> anyhow::Result<()> {
    let conn = db_pool.get().await?;
    // TODO: deal with this unwrap?
    let conn = conn.lock().unwrap();
    let mut stmt = conn.prepare_cached("DELETE FROM signal_chats WHERE id = $1")?;
    stmt.execute([id])?;

    Ok(())
}
