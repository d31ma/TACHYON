from typing import Annotated, Iterator
from models import Ticket


@request_route("/events")
@public
class EventsController:
    tickets: Annotated[TicketService, needs]
    @stream_route
    def stream_events(self) -> Iterator[Ticket]:
        yield from self._tickets_for(self.tickets.levels())

    def _tickets_for(self, levels: list[int]) -> Iterator[Ticket]:
        for level in levels:
            yield Ticket("anvil", level)
