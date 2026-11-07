from typing import Annotated

@request_route("/events")
@public
class EventsController:
    event_service: Annotated[EventService, needs]
    @get_route
    def get_events(self) -> dict[str, object]:
        return self.event_service.report()
