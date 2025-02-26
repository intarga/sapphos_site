use super::{Agenda, AgendaMonth, Event};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, Utc};
use chrono_tz::Europe::Oslo;
use itertools::Itertools;
use pulldown_cmark::Parser;
use serde::{Deserialize, Deserializer};
use std::sync::{Arc, RwLock};
use tracing::{error, info};

struct TaggedEvent {
    event: Event,
    month: String,
}

impl<'de> Deserialize<'de> for TaggedEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawEvent {
            summary: String,
            description: Option<String>,
            location: Option<String>,
            start: RawTime,
            end: RawTime,
        }

        #[derive(Deserialize)]
        struct RawTime {
            #[serde(rename = "dateTime")]
            date_time: DateTime<Utc>,
        }

        let raw_event = RawEvent::deserialize(deserializer)?;

        let start = raw_event.start.date_time.with_timezone(&Oslo);
        let end = raw_event.end.date_time.with_timezone(&Oslo);

        let description = raw_event
            .description
            .map(|d| {
                let parser = Parser::new(&d);
                let mut output = String::new();
                pulldown_cmark::html::push_html(&mut output, parser);
                output
            })
            .unwrap_or_else(|| "".to_string());

        let weekday = start.format("%A");
        let day_of_month = start.day();
        let ordinal = match day_of_month {
            1 | 21 | 31 => "st",
            2 | 22 => "nd",
            3 | 23 => "rd",
            _ => "th",
        };
        Ok(TaggedEvent {
            event: Event {
                title: raw_event.summary,
                description,
                location: raw_event.location.unwrap_or_else(|| String::from("")),
                start_time: start.format("%R").to_string(),
                end_time: end.format("%R").to_string(),
                day: format!("{} the {}{}", weekday, day_of_month, ordinal),
            },
            month: start.format("%B").to_string(),
        })
    }
}

pub async fn fetch_calendar() -> Result<Agenda> {
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

    let events: Vec<TaggedEvent> = serde_json::from_value(
        resp.get_mut("items")
            .ok_or_else(|| anyhow!("GCal JSON resonse did not contain key \"items\""))?
            .take(),
    )?;

    // FIXME: this is crimes
    Ok(events
        .into_iter()
        .enumerate()
        .chunk_by(|x| x.1.month.clone())
        .into_iter()
        .map(|group| {
            let (events, months): (Vec<(usize, Event)>, Vec<String>) = group
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

pub async fn refresh_agenda_at_interval(
    background_agenda: Arc<RwLock<Vec<AgendaMonth>>>,
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
