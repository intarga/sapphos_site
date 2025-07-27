use anyhow::anyhow;
use chrono::{Datelike, NaiveDate, NaiveTime};
use itertools::Itertools;
use pulldown_cmark::Parser;
use serde::Deserialize;

/// base event type that matches the forms and db schema
#[derive(Clone, Debug, Deserialize)]
pub struct Event {
    pub title: String,
    pub location: Option<String>,
    pub description: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub host: Option<String>,
    pub host_email: Option<String>,
}

/// what's needed to show an event on the admin page
#[derive(Clone, Debug, Deserialize)]
pub struct AdminEvent {
    pub id: i64,
    pub title: String,
    pub date: NaiveDate,
}

/// helper type used in construction of agenda
pub struct TaggedEvent {
    pub event: AgendaEvent,
    pub month: String,
}

#[derive(Clone, Debug)]
pub struct AgendaEvent {
    pub title: String,
    pub description: String,
    pub location: String,
    pub start_time: String,
    pub end_time: String,
    pub day: String,
}

#[derive(Clone, Debug)]
pub struct AgendaMonth {
    pub month: String,
    // the usize is an index used to decide how to colour the event divider
    pub events: Vec<(usize, AgendaEvent)>,
}

/// filtered and stylised list of events for rendering on the main page
pub type Agenda = Vec<AgendaMonth>;

// Event with some formatting changes for use in the event pages
#[derive(Clone, Debug)]
pub struct PageEvent {
    pub title: String,
    pub location: Option<String>,
    pub description: Option<String>,
    pub datetime: String,
    //pub host: Option<String>,
    //pub host_email: Option<String>,
}

impl From<Event> for PageEvent {
    fn from(event: Event) -> PageEvent {
        // TODO: should do on intake instead?
        let description = event.description.map(|desc| {
            let parser = Parser::new(&desc);
            let mut output = String::new();
            pulldown_cmark::html::push_html(&mut output, parser);
            output
        });
        // let description = {
        //     let description = self.description.unwrap_or_default();
        //     let parser = Parser::new(&description);
        //     let mut output = String::new();
        //     pulldown_cmark::html::push_html(&mut output, parser);
        //     output
        // };

        let weekday = event.start_date.format("%A");
        let day_of_month = event.start_date.day();
        let ordinal = match day_of_month {
            1 | 21 | 31 => "st",
            2 | 22 => "nd",
            3 | 23 => "rd",
            _ => "th",
        };
        let month_and_year = event.start_date.format("%B %Y").to_string();
        let start_time = event
            .start_time
            .map(|time| time.format(", %H:%M").to_string())
            .unwrap_or_else(|| "".to_string());
        let end_time = event
            .end_time
            .map(|time| time.format(" - %H:%M").to_string())
            .unwrap_or_else(|| "".to_string());

        let datetime = format!(
            "{weekday} the {day_of_month}{ordinal} of {month_and_year}{start_time}{end_time}"
        );

        PageEvent {
            title: event.title,
            location: event.location,
            description,
            datetime,
        }
    }
}

pub async fn select_event(db_pool: deadpool_sqlite::Pool, id: i64) -> anyhow::Result<Event> {
    let conn = db_pool.get().await?;
    let event = conn
        .interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                r#"
                SELECT
                    title,
                    location,
                    description,
                    start_date,
                    end_date,
                    start_time,
                    end_time,
                    host,
                    host_email
                FROM events
                WHERE id = $1
                "#,
            )?;
            stmt.query_row([id], |row| {
                Ok(Event {
                    title: row.get(0)?,
                    location: row.get(1)?,
                    description: row.get(2)?,
                    start_date: row.get(3)?,
                    end_date: row.get(4)?,
                    start_time: row.get(5)?,
                    end_time: row.get(6)?,
                    host: row.get(7)?,
                    host_email: row.get(8)?,
                })
            })
        })
        .await
        .unwrap()?;

    Ok(event)
}

pub async fn select_events(db_pool: deadpool_sqlite::Pool) -> anyhow::Result<Vec<Event>> {
    let conn = db_pool.get().await?;
    let events = conn
        .interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                r#"
                SELECT
                    title,
                    location,
                    description,
                    start_date,
                    end_date,
                    start_time,
                    end_time,
                    host,
                    host_email
                FROM events
                WHERE start_date BETWEEN
                    date('now','-1 day') AND
                    date('now','-1 day','start of month','+1 year', '-1 day')
                ORDER BY start_date ASC
                "#,
            )?;
            #[allow(clippy::let_and_return)]
            let events = stmt
                .query_map([], |row| {
                    Ok(Event {
                        title: row.get(0)?,
                        location: row.get(1)?,
                        description: row.get(2)?,
                        start_date: row.get(3)?,
                        end_date: row.get(4)?,
                        start_time: row.get(5)?,
                        end_time: row.get(6)?,
                        host: row.get(7)?,
                        host_email: row.get(8)?,
                    })
                })?
                .map(|res| res.map_err(|e| anyhow!(e)))
                .collect::<Result<Vec<Event>, anyhow::Error>>();

            events
        })
        .await
        .unwrap()?;

    Ok(events)
}

pub async fn make_agenda(db_pool: deadpool_sqlite::Pool) -> anyhow::Result<Agenda> {
    // TODO: remove when we're done using gcal
    let events = select_events(db_pool).await?;

    let tagged_events: Vec<TaggedEvent> = events
        .into_iter()
        .map(|event| {
            // TODO: should do on intake instead?
            let description = {
                let description = event.description.unwrap_or_default();
                let parser = Parser::new(&description);
                let mut output = String::new();
                pulldown_cmark::html::push_html(&mut output, parser);
                output
            };

            let weekday = event.start_date.format("%A");
            let day_of_month = event.start_date.day();
            let ordinal = match day_of_month {
                1 | 21 | 31 => "st",
                2 | 22 => "nd",
                3 | 23 => "rd",
                _ => "th",
            };

            TaggedEvent {
                event: AgendaEvent {
                    title: event.title,
                    description,
                    location: event.location.unwrap_or_default(),
                    start_time: event
                        .start_time
                        .map(|time| time.format("%H:%M").to_string())
                        .unwrap_or_else(|| "––:––".to_string()),
                    end_time: event
                        .end_time
                        .map(|time| time.format("%H:%M").to_string())
                        .unwrap_or_else(|| "––:––".to_string()),
                    day: format!("{} the {}{}", weekday, day_of_month, ordinal),
                },
                month: event.start_date.format("%B").to_string(),
            }
        })
        .collect();

    // FIXME: this is crimes
    Ok(tagged_events
        .into_iter()
        .enumerate()
        .chunk_by(|x| x.1.month.clone())
        .into_iter()
        .map(|group| {
            let (events, months): (Vec<(usize, AgendaEvent)>, Vec<String>) = group
                .1
                .map(|tagged| ((tagged.0, tagged.1.event), tagged.1.month))
                .unzip();
            AgendaMonth {
                month: months.first().unwrap().clone(),
                events,
            }
        })
        .collect())
}

pub async fn select_admin_events(
    db_pool: deadpool_sqlite::Pool,
) -> anyhow::Result<Vec<AdminEvent>> {
    let conn = db_pool.get().await?;
    let events = conn
        .interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT id, title, start_date FROM events ORDER BY start_date ASC",
            )?;
            #[allow(clippy::let_and_return)]
            let events = stmt
                .query_map([], |row| {
                    Ok(AdminEvent {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        date: row.get(2)?,
                    })
                })?
                .map(|res| res.map_err(|e| anyhow!(e)))
                .collect::<Result<Vec<AdminEvent>, anyhow::Error>>();

            events
        })
        .await
        .unwrap()?;

    Ok(events)
}

pub async fn insert_event(db_pool: deadpool_sqlite::Pool, event: Event) -> anyhow::Result<()> {
    let conn = db_pool.get().await?;
    conn.interact(move |conn| {
        let mut stmt = conn.prepare_cached(
            r#"
                INSERT INTO events (
                    title,
                    location,
                    description,
                    start_date,
                    end_date,
                    start_time,
                    end_time,
                    host,
                    host_email)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
        )?;
        stmt.execute(rusqlite::params![
            event.title,
            event.location,
            event.description,
            event.start_date,
            event.end_date,
            event.start_time,
            event.end_time,
            event.host,
            event.host_email,
        ])
    })
    .await
    .unwrap()?;

    Ok(())
}

pub async fn update_event(
    db_pool: deadpool_sqlite::Pool,
    id: i64,
    event: Event,
) -> anyhow::Result<()> {
    let conn = db_pool.get().await?;
    conn.interact(move |conn| {
        let mut stmt = conn.prepare_cached(
            r#"
                 UPDATE events SET
                    title = $1,
                    location = $2,
                    description = $3,
                    start_date = $4,
                    end_date = $5,
                    start_time = $6,
                    end_time = $7,
                    host = $8,
                    host_email = $9
                WHERE id = $10
                "#,
        )?;
        stmt.execute(rusqlite::params![
            event.title,
            event.location,
            event.description,
            event.start_date,
            event.end_date,
            event.start_time,
            event.end_time,
            event.host,
            event.host_email,
            id,
        ])
    })
    .await
    .unwrap()?;

    Ok(())
}

pub async fn delete_event(db_pool: deadpool_sqlite::Pool, id: i64) -> anyhow::Result<()> {
    let conn = db_pool.get().await?;
    conn.interact(move |conn| {
        let mut stmt = conn.prepare_cached("DELETE FROM events WHERE id = $1")?;
        stmt.execute([id])
    })
    .await
    .unwrap()?;

    Ok(())
}
