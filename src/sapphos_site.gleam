import birl.{type Time}
import gleam/dynamic
import gleam/io
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/result
import lustre
import lustre/attribute
import lustre/effect.{type Effect}
import lustre/element.{type Element}
import lustre/element/html
import lustre_http.{type HttpError}

type Event {
  Event(summary: String, description: Option(String), start_time: Time)
}

type Model {
  Model(events: List(Event))
}

pub opaque type Msg {
  ApiReturnedEvents(Result(List(Event), HttpError))
}

fn init(_flags) -> #(Model, Effect(Msg)) {
  #(Model(events: []), get_events())
}

fn get_events() -> Effect(Msg) {
  // TODO: specify timeMin dynamically
  let url =
    "https://www.googleapis.com/calendar/v3/calendars/1fa82a44ca905662ca167d3d3d28b9c696852f5838be661d3d5b1de552e261bc%40group.calendar.google.com/events?key=AIzaSyD77xGddvaY1SYANkCwFF5yw3mfxt303no&singleEvents=True&orderBy=startTime&timeMin=2024-10-10T00:00:00Z"

  let decoder =
    dynamic.field(
      "items",
      dynamic.list(fn(dyn) {
        // io.debug(dyn)
        dynamic.decode3(
          Event,
          dynamic.field("summary", dynamic.string),
          dynamic.optional_field("description", dynamic.string),
          dynamic.field("start", fn(dyn) {
            use dt_string <- result.try(dynamic.field(
              "dateTime",
              dynamic.string,
            )(dyn))
            result.map_error(birl.parse(dt_string), fn(_) {
              [dynamic.DecodeError("datetime", dt_string, [""])]
            })
          }),
        )(dyn)
      }),
    )

  lustre_http.get(url, lustre_http.expect_json(decoder, ApiReturnedEvents))
}

fn update(model: Model, msg: Msg) -> #(Model, Effect(Msg)) {
  io.debug(msg)

  case msg {
    ApiReturnedEvents(Ok(events)) -> #(Model(events), effect.none())
    // TODO
    ApiReturnedEvents(Error(_)) -> {
      // io.debug(e)
      #(model, effect.none())
    }
  }
}

fn view_events(events: List(Event)) -> Element(msg) {
  html.div(
    [attribute.class("event-list")],
    list.map(events, fn(event) {
      io.debug(event.start_time)
      html.div([attribute.class("event")], [
        element.text(event.summary),
        element.text(
          event.start_time
          |> birl.get_time_of_day
          |> birl.time_of_day_to_short_string,
        ),
        html.br([]),
        element.text(case event.description {
          Some(description) -> description
          None -> ""
        }),
      ])
    }),
  )
}

fn view(model: Model) -> Element(Msg) {
  view_events(model.events)
}

pub fn main() {
  let app = lustre.application(init, update, view)
  let assert Ok(_) = lustre.start(app, "#app", Nil)
}
