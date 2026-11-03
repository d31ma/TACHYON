from typing import Annotated, ClassVar

from models import Tick


@service
class StreamService:
    _TICK_LIMIT: ClassVar[int] = 128

    ticks_client: Annotated[TicksClient, needs]
    ticks: list[Tick] = []

    @on_event
    def add_tick(self, tick: Tick) -> None:
        self.ticks = (self.ticks + [tick])[-self._TICK_LIMIT:]
