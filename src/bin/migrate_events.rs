use sapphos_site::events::{gcal::fetch_calendar, insert_event, Event};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let deadpool_cfg = deadpool_sqlite::Config::new("db/db.sqlite3");
    let db_pool = deadpool_cfg
        .create_pool(deadpool_sqlite::Runtime::Tokio1)
        .expect("Failed to create DB pool");

    let events: Vec<Event> = fetch_calendar().await?;

    for event in events {
        insert_event(db_pool.clone(), event).await?;
    }

    Ok(())
}
