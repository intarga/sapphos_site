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
    start_time: Time,
    end_time: Time,
  )
}

type Event {
  Event(
    summary: String,
    description: Option(String),
    start_time: String,
    end_time: String,
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

fn get_events() -> Effect(Msg) {
  // TODO: specify timeMin dynamically
  let url =
    "https://www.googleapis.com/calendar/v3/calendars/1fa82a44ca905662ca167d3d3d28b9c696852f5838be661d3d5b1de552e261bc%40group.calendar.google.com/events?key=AIzaSyD77xGddvaY1SYANkCwFF5yw3mfxt303no&singleEvents=True&orderBy=startTime&timeMin=2024-10-10T00:00:00Z"

  let decoder =
    dynamic.field(
      "items",
      dynamic.list(dynamic.decode4(
        RawEvent,
        dynamic.field("summary", dynamic.string),
        dynamic.optional_field("description", dynamic.string),
        dynamic.field("start", decode_datetime),
        dynamic.field("end", decode_datetime),
      )),
    )

  lustre_http.get(url, lustre_http.expect_json(decoder, ApiReturnedEvents))
}

fn init(_flags) -> #(Model, Effect(Msg)) {
  #(Model(agenda: []), get_events())
}

fn format_date(raw_event: RawEvent) -> String {
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

fn process_event(raw_event: RawEvent) -> Event {
  let start_time =
    raw_event.start_time
    |> birl.get_time_of_day
    |> birl.time_of_day_to_short_string
  let end_time =
    raw_event.end_time
    |> birl.get_time_of_day
    |> birl.time_of_day_to_short_string
  Event(raw_event.summary, raw_event.description, start_time, end_time)
}

fn process_event_list(raw_events: List(RawEvent)) -> List(AgendaDay) {
  raw_events
  |> list.chunk(fn(raw_event) { birl.get_day(raw_event.start_time) })
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

fn view_event(event: Event) -> Element(Msg) {
  html.div([attribute.class("event")], [
    html.div([attribute.class("event-header")], [
      html.div([attribute.class("event-title")], [element.text(event.summary)]),
      html.div([attribute.class("event-start")], [
        element.text(event.start_time),
      ]),
      html.div([attribute.class("event-loc")], [
        element.text("location placeholder"),
      ]),
      html.div([attribute.class("event-end")], [element.text(event.end_time)]),
    ]),
    html.div([attribute.class("event-text")], [
      element.text(case event.description {
        Some(description) -> description
        None -> ""
      }),
    ]),
  ])
}

fn view_event_day(event_day: AgendaDay) -> Element(Msg) {
  html.div([attribute.class("agenda-day")], [
    html.div([attribute.class("agenda-day-header")], [
      element.text(event_day.date),
    ]),
    html.div(
      [attribute.class("event-list")],
      list.map(event_day.events, view_event),
    ),
  ])
}

fn view_agenda(calendar: Agenda) -> Element(Msg) {
  io.debug(calendar)
  html.div([attribute.class("agenda")], list.map(calendar, view_event_day))
}

fn view(model: Model) -> Element(Msg) {
  view_agenda(model.agenda)
}

pub fn main() {
  let app = lustre.application(init, update, view)
  let assert Ok(_) = lustre.start(app, "#app", Nil)
}
