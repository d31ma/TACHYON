@RequestRoute("/events")
@Public
class EventsController {
  @Needs
  val eventService: EventService
  @GetRoute
  fun getEvents(): Any? = eventService.report()
}
