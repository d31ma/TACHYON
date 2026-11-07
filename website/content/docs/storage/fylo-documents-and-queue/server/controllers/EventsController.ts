@RequestRoute("/events")
@Public
class EventsController {
  @Needs
  eventService!: EventService;
  @GetRoute
  getEvents() { return this.eventService.report(); }
}
