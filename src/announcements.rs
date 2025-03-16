use anyhow::anyhow;
use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Announcement {
    pub title: String,
    pub body: String,
    pub date: NaiveDate,
    pub author: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AdminAnnouncement {
    pub id: i64,
    pub title: String,
    pub date: NaiveDate,
}

pub async fn select_announcement(
    db_pool: deadpool_sqlite::Pool,
    id: i64,
) -> anyhow::Result<Announcement> {
    let conn = db_pool.get().await?;
    // TODO: deal with this unwrap?
    let conn = conn.lock().unwrap();
    let mut stmt =
        conn.prepare_cached("SELECT title, body, date, author FROM announcements WHERE id = $1")?;
    let announcement = stmt.query_row([id], |row| {
        Ok(Announcement {
            title: row.get(0)?,
            body: row.get(1)?,
            date: row.get(2)?,
            author: row.get(3)?,
        })
    })?;

    Ok(announcement)
}

pub async fn select_announcements(
    db_pool: deadpool_sqlite::Pool,
) -> anyhow::Result<Vec<Announcement>> {
    let conn = db_pool.get().await?;
    // TODO: deal with this unwrap?
    let conn = conn.lock().unwrap();
    let mut stmt = conn
        .prepare_cached("SELECT title, body, date, author FROM announcements ORDER BY id DESC")?;
    let announcements = stmt
        .query_map([], |row| {
            Ok(Announcement {
                title: row.get(0)?,
                body: row.get(1)?,
                date: row.get(2)?,
                author: row.get(3)?,
            })
        })?
        .map(|res| res.map_err(|e| anyhow!(e)))
        .collect::<Result<Vec<Announcement>, anyhow::Error>>()?;

    Ok(announcements)
}

pub async fn select_admin_announcements(
    db_pool: deadpool_sqlite::Pool,
) -> anyhow::Result<Vec<AdminAnnouncement>> {
    let conn = db_pool.get().await?;
    // TODO: deal with this unwrap?
    let conn = conn.lock().unwrap();
    let mut stmt =
        conn.prepare_cached("SELECT id, title, date FROM announcements ORDER BY id DESC")?;
    let announcements = stmt
        .query_map([], |row| {
            Ok(AdminAnnouncement {
                id: row.get(0)?,
                title: row.get(1)?,
                date: row.get(2)?,
            })
        })?
        .map(|res| res.map_err(|e| anyhow!(e)))
        .collect::<Result<Vec<AdminAnnouncement>, anyhow::Error>>()?;

    Ok(announcements)
}

pub async fn insert_announcement(
    db_pool: deadpool_sqlite::Pool,
    announcement: Announcement,
) -> anyhow::Result<()> {
    let conn = db_pool.get().await?;
    // TODO: deal with this unwrap?
    let conn = conn.lock().unwrap();
    let mut stmt = conn.prepare_cached(
        "INSERT INTO announcements (title, body, date, author) VALUES ($1, $2, $3, $4)",
    )?;
    stmt.execute(rusqlite::params![
        announcement.title,
        announcement.body,
        announcement.date,
        announcement.author,
    ])?;

    Ok(())
}

pub async fn update_announcement(
    db_pool: deadpool_sqlite::Pool,
    id: i64,
    announcement: Announcement,
) -> anyhow::Result<()> {
    let conn = db_pool.get().await?;
    // TODO: deal with this unwrap?
    let conn = conn.lock().unwrap();
    let mut stmt = conn.prepare_cached(
        "UPDATE announcements SET title = $1, body = $2, date = $3, author = $4 WHERE id = $5",
    )?;
    stmt.execute(rusqlite::params![
        announcement.title,
        announcement.body,
        announcement.date,
        announcement.author,
        id,
    ])?;

    Ok(())
}

pub async fn delete_announcement(db_pool: deadpool_sqlite::Pool, id: i64) -> anyhow::Result<()> {
    let conn = db_pool.get().await?;
    // TODO: deal with this unwrap?
    let conn = conn.lock().unwrap();
    let mut stmt = conn.prepare_cached("DELETE FROM announcements WHERE id = $1")?;
    stmt.execute([id])?;

    Ok(())
}
