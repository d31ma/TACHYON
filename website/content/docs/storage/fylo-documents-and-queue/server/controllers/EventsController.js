@RequestRoute("/events")
@Public
class EventsController {
  /** @type {EventService} */
  @Needs
  eventService;
  /** @returns {Promise<object>} */
  @GetRoute
  getEvents() { return this.eventService.report(); }
}
