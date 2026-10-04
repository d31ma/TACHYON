@RequestRoute("/events")
@Public
class EventsController {
  @Needs
  final EventService eventService;

  @GetRoute
  Future<Map<String, Object?>> getEvents() {
    return eventService.report();
  }
}
