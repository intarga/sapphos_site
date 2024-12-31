const divider_styles = [
  "background-color: var(--rainbow-1);",
  "background-color: var(--rainbow-2);",
  "background-color: var(--rainbow-3);",
  "background-color: var(--rainbow-4);",
  "background-color: var(--rainbow-5);",
  "background-color: var(--rainbow-6);",
]

const fetch_calendar = async () => {
  const calendar_id =
    "1fa82a44ca905662ca167d3d3d28b9c696852f5838be661d3d5b1de552e261bc%40group.calendar.google.com";
  const key = "AIzaSyD77xGddvaY1SYANkCwFF5yw3mfxt303no";
  const time_min = new Date().toISOString();
  const url = [
    "https://www.googleapis.com/calendar/v3/calendars/",
    calendar_id,
    "/events?key=",
    key,
    "&singleEvents=True&orderBy=startTime&timeMin=",
    time_min,
  ].join('');

  const response = await fetch(url);
  if (!response.ok) {
    console.error("Failed to fetch from Google Calendar API: ", response.status);
  }
  const json = await response.json();

  return json;
}

const process_calendar = (raw_calendar) => {
  const events = raw_calendar.items.map((item, i) => ({
    title: item.summary,
    description: (item.hasOwnProperty("description") ? item.description : ""),
    location: (item.hasOwnProperty("location") ? item.location : ""),
    start: new Date(item.start.dateTime),
    end: new Date(item.end.dateTime),
    color_index: i,
  }));

  let months = [];
  let current_month = {month: -1, events: []};
  for (const event of events) {
    let event_month = event.start.getMonth();

    if (event_month !== current_month.month) {
      if (current_month.month === -1) {
        current_month.month = event_month;
      } else {
        months.push(current_month);
        current_month = {month: event_month, events: []};
      }
    }

    current_month.events.push(event);
  }
  months.push(current_month);

  return months
}

const ordinal = (n) => {
  if (n > 3 && n < 21) {
    return "th";
  }
  switch (n % 10) {
    case 1: return "st";
    case 2: return "nd";
    case 3: return "rd";
    default: return "th";
  }
}

const render_agenda = (months) => {
  const month_formatter = new Intl.DateTimeFormat("en-us", { month: "long" });
  const time_formatter = new Intl.DateTimeFormat("en-us", { hour: "numeric", minute: "numeric", hour12: false });
  const day_formatter = new Intl.DateTimeFormat("en-us", { weekday: "long", day: "numeric" });

  const parent = document.getElementById("month-list");
  
  const month_elements = months.map((month) => {
    const month_element = document.createElement("div");
    month_element.className = "agenda-month";
    
    const header = document.createElement("div");
    header.className = "agenda-month-header";
    // TODO: handle case where first event doesn't exist
    header.textContent = month_formatter.format(month.events[0].start);

    const hr = document.createElement("hr");

    const event_list = document.createElement("div");
    event_list.className = "event-list";
    const event_elements = month.events.map((event) => {
      const event_element = document.createElement("details");
      event_element.className = "event";

      const summary = document.createElement("summary");
      summary.className = "event-summary";

      const title = document.createElement("div");
      title.className = "event-title";
      title.textContent = event.title;

      const header = document.createElement("div");
      header.className = "event-header";

      const time = document.createElement("div");
      time.className = "event-time";

      const start = document.createElement("div");
      start.className = "event-start";
      start.textContent = time_formatter.format(event.start);

      const end = document.createElement("div");
      end.className = "event-end";
      end.textContent = time_formatter.format(event.end);

      time.append(start, end);

      const divider = document.createElement("div");
      divider.className = "event-header-divider";
      divider.style = divider_styles[event.color_index % 6];
      
      const short_info = document.createElement("div");
      short_info.className = "event-short-info";

      const day = document.createElement("div");
      day.className = "event-day";
      const formatted_parts = day_formatter.formatToParts(event.start);
      const weekday = formatted_parts[0];
      const date = formatted_parts[2];
      const day_text = [weekday.value, " the ", date.value, ordinal(date.value)].join('')
      day.textContent = day_text;

      const loc = document.createElement("div");
      loc.className = "event-loc";
      loc.textContent = event.location;

      short_info.append(day, loc);

      header.append(time, divider, short_info);

      summary.append(title, header);

      const description = document.createElement("div");
      description.className = "event-text";
      description.textContent = event.description;

      event_element.append(summary, description);

      return event_element;
    });
    event_list.append(...event_elements)

    month_element.append(header, hr, event_list);

    return month_element
  });

  parent.append(...month_elements);
}

// TODO: figure out how to load the data earlier
document.addEventListener("DOMContentLoaded", async function(){
    const raw_calendar = await fetch_calendar();
    const calendar = process_calendar(raw_calendar);
    render_agenda(calendar);
});
