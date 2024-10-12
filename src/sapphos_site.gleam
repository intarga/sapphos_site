import birl.{type Time}
import gleam/dynamic
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/result
import gleam/string
import lustre
import lustre/attribute
import lustre/effect.{type Effect}
import lustre/element.{type Element}
import lustre/element/html
import lustre_http.{type HttpError}

type RawEvent {
  RawEvent(
    summary: String,
    description: Option(String),
    location: Option(String),
    start_time: Time,
    end_time: Time,
  )
}

type Event {
  Event(
    summary: String,
    description: Option(String),
    location: Option(String),
    start_time: String,
    end_time: String,
    color_index: Int,
  )
}

type AgendaDay {
  AgendaDay(date: String, events: List(Event))
}

type Agenda =
  List(AgendaDay)

type Model {
  Model(agenda: Agenda)
}

pub opaque type Msg {
  ApiReturnedEvents(Result(List(RawEvent), HttpError))
}

fn decode_datetime(dyn: dynamic.Dynamic) {
  use dt_string <- result.try(dynamic.field("dateTime", dynamic.string)(dyn))
  result.map_error(birl.parse(dt_string), fn(_) {
    [dynamic.DecodeError("datetime", dt_string, [""])]
  })
}

fn now_rfc3339() -> String {
  let iso = birl.now() |> birl.to_iso8601
  let assert Ok(#(pre_dot, _post_dot)) = string.split_once(iso, ".")
  // let #(post_sign, tz_sign) = case string.split_once(post_dot, "+") {
  //   Ok(#(_, post_sign)) -> #(post_sign, "+")
  //   Error(_) -> {
  //     let assert Ok(#(_, post_sign)) = string.split_once(post_dot, "-")
  //     #(post_sign, "-")
  //   }
  // }
  // FIXME: Google's API apparently doesn't like non UTC stamps???
  // This isn't quite correct because we just removed the time offset, really
  // we should calculate a correct UTC timestamp, but birl doesn't seem to
  // support that, so I guess we have to make our own formatter, or fix tempo
  pre_dot <> "Z"
  //tz_sign <> post_sign
}

fn get_events() -> Effect(Msg) {
  let calendar_id =
    "1fa82a44ca905662ca167d3d3d28b9c696852f5838be661d3d5b1de552e261bc%40group.calendar.google.com"
  let key = "AIzaSyD77xGddvaY1SYANkCwFF5yw3mfxt303no"
  let time_min = now_rfc3339()
  let url =
    "https://www.googleapis.com/calendar/v3/calendars/"
    <> calendar_id
    <> "/events?key="
    <> key
    <> "&singleEvents=True&orderBy=startTime&timeMin="
    <> time_min

  let decoder =
    dynamic.field(
      "items",
      dynamic.list(dynamic.decode5(
        RawEvent,
        dynamic.field("summary", dynamic.string),
        dynamic.optional_field("description", dynamic.string),
        dynamic.optional_field("location", dynamic.string),
        dynamic.field("start", decode_datetime),
        dynamic.field("end", decode_datetime),
      )),
    )

  lustre_http.get(url, lustre_http.expect_json(decoder, ApiReturnedEvents))
}

fn init(_flags) -> #(Model, Effect(Msg)) {
  #(Model(agenda: []), get_events())
}

fn format_date(raw_event: #(RawEvent, Int)) -> String {
  let raw_event = raw_event.0
  let day_of_week =
    raw_event.start_time
    |> birl.weekday
    |> birl.weekday_to_short_string
    |> string.uppercase
  let date = birl.get_day(raw_event.start_time).date |> int.to_string
  let month = birl.short_string_month(raw_event.start_time) |> string.uppercase
  // let year = birl.get_day(raw_event.start_time).year |> int.to_string
  day_of_week <> " " <> date <> " " <> month
  //<> " " <> year
}

fn process_event(raw_event: #(RawEvent, Int)) -> Event {
  let color_index = raw_event.1
  let raw_event = raw_event.0
  let start_time =
    raw_event.start_time
    |> birl.get_time_of_day
    |> birl.time_of_day_to_short_string
  let end_time =
    raw_event.end_time
    |> birl.get_time_of_day
    |> birl.time_of_day_to_short_string
  Event(
    raw_event.summary,
    raw_event.description,
    raw_event.location,
    start_time,
    end_time,
    color_index,
  )
}

fn process_event_list(raw_events: List(RawEvent)) -> List(AgendaDay) {
  raw_events
  |> list.zip(list.range(0, list.length(raw_events)))
  |> list.chunk(fn(raw_event) { birl.get_day({ raw_event.0 }.start_time) })
  |> list.map(fn(raw_events) {
    let assert Ok(date) =
      list.first(raw_events)
      |> result.map(format_date)
    let events =
      raw_events
      |> list.map(process_event)
    AgendaDay(date, events)
  })
}

fn update(model: Model, msg: Msg) -> #(Model, Effect(Msg)) {
  case msg {
    ApiReturnedEvents(Ok(raw_events)) -> #(
      Model(process_event_list(raw_events)),
      effect.none(),
    )
    ApiReturnedEvents(Error(e)) -> {
      io.debug(e)
      #(model, effect.none())
    }
  }
}

// const divider_colors = ["--salmon-pink", "--sky-blue"]
const divider_colors = [
  "--rainbow-1", "--rainbow-2", "--rainbow-3", "--rainbow-4", "--rainbow-5",
  "--rainbow-6",
]

fn view_event(event: Event) -> Element(Msg) {
  let assert Ok(divider_color) =
    divider_colors
    |> list.drop(event.color_index % list.length(divider_colors))
    |> list.take(1)
    |> list.first
  html.details([attribute.class("event")], [
    html.summary([attribute.class("event-header")], [
      html.div([attribute.class("event-time")], [
        html.div([attribute.class("event-start")], [
          element.text(event.start_time),
        ]),
        html.div([attribute.class("event-end")], [element.text(event.end_time)]),
      ]),
      html.div(
        [
          attribute.class("event-header-divider"),
          attribute.style([
            #("background-color", "var(" <> divider_color <> ")"),
          ]),
        ],
        [],
      ),
      html.div([attribute.class("event-short-info")], [
        html.div([attribute.class("event-title")], [element.text(event.summary)]),
        html.div([attribute.class("event-loc")], [
          element.text(case event.location {
            Some(location) -> location
            None -> ""
          }),
        ]),
      ]),
    ]),
    html.div(
      [
        attribute.class("event-text"),
        // attribute.property("innerHTML", case event.description {
      //   Some(description) -> description
      //   None -> ""
      // }),
      ],
      [
        element.text(case event.description {
          Some(description) -> description
          None -> ""
        }),
      ],
    ),
  ])
}

fn view_event_day(event_day: AgendaDay) -> Element(Msg) {
  html.div([attribute.class("agenda-day")], [
    html.div([attribute.class("agenda-day-header")], [
      element.text(event_day.date),
    ]),
    html.hr([]),
    html.div(
      [attribute.class("event-list")],
      list.map(event_day.events, view_event),
    ),
  ])
}

fn view_agenda(calendar: Agenda) -> Element(Msg) {
  html.div([attribute.class("agenda")], list.map(calendar, view_event_day))
}

fn view(model: Model) -> Element(Msg) {
  view_agenda(model.agenda)
}

pub fn main() {
  let app = lustre.application(init, update, view)
  let assert Ok(_) = lustre.start(app, "#app", Nil)
}
