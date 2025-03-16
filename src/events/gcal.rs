use crate::events::Event;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use chrono_tz::Europe::Oslo;
use serde::Deserialize;
use std::sync::{Arc, RwLock};
use tracing::{error, info};

#[derive(Deserialize, PartialEq, Eq)]
struct RawTime {
    #[serde(rename = "dateTime")]
    date_time: DateTime<Utc>,
}

#[derive(Deserialize)]
struct RawEvent {
    summary: String,
    description: Option<String>,
    location: Option<String>,
    start: RawTime,
    end: RawTime,
}

fn process_event(raw_event: RawEvent) -> Event {
    let start_date = raw_event.start.date_time.with_timezone(&Oslo).date_naive();
    let end_date = if raw_event.start == raw_event.end {
        None
    } else {
        Some(raw_event.end.date_time.with_timezone(&Oslo).date_naive())
    };

    Event {
        title: raw_event.summary,
        location: raw_event.location,
        description: raw_event.description,
        start_date,
        end_date,
        start_time: Some(raw_event.start.date_time.with_timezone(&Oslo).time()),
        end_time: Some(raw_event.end.date_time.with_timezone(&Oslo).time()),
        host: None,
        host_email: None,
    }
}

pub async fn fetch_calendar() -> Result<Vec<Event>> {
    let client = reqwest::Client::new();

    let calendar_id =
    "1fa82a44ca905662ca167d3d3d28b9c696852f5838be661d3d5b1de552e261bc%40group.calendar.google.com";
    let key = "AIzaSyD77xGddvaY1SYANkCwFF5yw3mfxt303no";
    let time_min = Utc::now();
    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/{}/events",
        calendar_id
    );

    let mut resp: serde_json::Value = client
        .get(url)
        .query(&[
            ("key", key),
            ("singleEvents", "True"),
            ("orderBy", "startTime"),
            ("timeMin", &time_min.to_rfc3339()),
        ])
        .send()
        .await?
        .json()
        .await?;

    let raw_events: Vec<RawEvent> = serde_json::from_value(
        resp.get_mut("items")
            .ok_or_else(|| anyhow!("GCal JSON resonse did not contain key \"items\""))?
            .take(),
    )?;

    Ok(raw_events.into_iter().map(process_event).collect())
}

pub async fn refresh_agenda_at_interval(
    background_agenda: Arc<RwLock<Vec<Event>>>,
    mut interval: tokio::time::Interval,
) {
    'refresh: loop {
        interval.tick().await;
        let new_agenda = match fetch_calendar().await {
            Ok(agenda) => agenda,
            Err(e) => {
                error!("Failed to refresh Agenda from GCal API: {}", e);
                continue 'refresh;
            }
        };
        let mut agenda = match background_agenda.write() {
            Ok(lock) => lock,
            Err(e) => {
                error!("Failed to acquire lock on background state: {}", e);
                continue 'refresh;
            }
        };
        *agenda = new_agenda;
        info!("Successfully refreshed agenda");
    }
}
