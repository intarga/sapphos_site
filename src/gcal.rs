use super::{Agenda, AgendaMonth, Event};
use chrono::{DateTime, Datelike, Utc};
use chrono_tz::Europe::Oslo;
use itertools::Itertools;
use serde::{Deserialize, Deserializer};

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
                description: raw_event.description.unwrap_or_else(|| String::from("")),
                location: raw_event.location.unwrap_or_else(|| String::from("")),
                start_time: start.format("%R").to_string(),
                end_time: end.format("%R").to_string(),
                day: format!("{} the {}{}", weekday, day_of_month, ordinal),
            },
            month: start.format("%B").to_string(),
        })
    }
}

pub async fn fetch_calendar() -> Agenda {
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
        .await
        // TODO: remove
        .unwrap()
        .json()
        .await
        // TODO: remove
        .unwrap();

    // TODO: remove unwraps
    let events: Vec<TaggedEvent> =
        serde_json::from_value(resp.get_mut("items").unwrap().take()).unwrap();

    // FIXME: this is crimes
    events
        .into_iter()
        .enumerate()
        .chunk_by(|x| x.1.month.clone())
        .into_iter()
        .map(|group| {
            let (events, months): (Vec<(usize, Event)>, Vec<String>) = group
                .1
                .into_iter()
                .map(|tagged| ((tagged.0, tagged.1.event), tagged.1.month))
                .unzip();
            AgendaMonth {
                month: months.first().unwrap().clone(),
                events,
            }
        })
        .collect()
}
