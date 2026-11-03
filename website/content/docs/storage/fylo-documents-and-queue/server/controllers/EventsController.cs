[RequestRoute("/events")]
[Public]
class EventsController {
  [Needs]
  EventService _eventService;
  [GetRoute]
  public object GetEvents() { return _eventService.Report(); }
}
