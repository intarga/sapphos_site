import gleam/dynamic
import gleam/io
import gleam/list
import gleam/option.{type Option}
import lustre
import lustre/attribute
import lustre/effect.{type Effect}
import lustre/element.{type Element}
import lustre/element/html
import lustre_http.{type HttpError}

type Event {
  Event(summary: String, description: Option(String))
}

type Model {
  Model(events: List(Event))
}

pub opaque type Msg {
  ApiReturnedEvents(Result(Model, HttpError))
}

fn init(_flags) -> #(Model, Effect(Msg)) {
  #(Model(events: []), get_events())
}

fn get_events() -> Effect(Msg) {
  // let url = "https://api.quotable.io/random"
  // TODO: specify timeMin and timeMax query params
  let url =
    "https://www.googleapis.com/calendar/v3/calendars/1fa82a44ca905662ca167d3d3d28b9c696852f5838be661d3d5b1de552e261bc%40group.calendar.google.com/events?key=AIzaSyD77xGddvaY1SYANkCwFF5yw3mfxt303no&singleEvents=True"

  // TODO: can we just decode a list of events here instead of the whole model?
  let decoder =
    dynamic.decode1(
      Model,
      dynamic.field(
        "items",
        dynamic.list(fn(dyn) {
          io.debug(dyn)
          dynamic.decode2(
            Event,
            dynamic.field("summary", dynamic.string),
            dynamic.optional_field("description", dynamic.string),
          )(dyn)
        }),
      ),
    )

  lustre_http.get(url, lustre_http.expect_json(decoder, ApiReturnedEvents))
}

fn update(model: Model, msg: Msg) -> #(Model, Effect(Msg)) {
  io.debug(msg)

  case msg {
    ApiReturnedEvents(Ok(new_model)) -> #(new_model, effect.none())
    // TODO
    ApiReturnedEvents(Error(_)) -> #(model, effect.none())
  }
}

fn view_events(events: List(Event)) -> Element(msg) {
  html.div(
    [attribute.class("event-list")],
    list.map(events, fn(event) {
      html.div([attribute.class("event")], [
        element.text(event.summary),
        // element.text(event.description),
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
