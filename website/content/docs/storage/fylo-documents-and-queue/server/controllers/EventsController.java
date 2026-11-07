@RequestRoute("/events")
@Public
class EventsController {
  @Needs
  EventService eventService;
  @GetRoute
  Object getEvents() { return eventService.report(); }
}
