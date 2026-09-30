from typing import Iterator

from models import Tick


@request_route("/ticks")
@public
class TicksController:
    @stream_route
    def stream_ticks(self) -> Iterator[Tick]:
        yield from self._ticks_up_to(3)

    def _ticks_up_to(self, last: int) -> Iterator[Tick]:
        for level in range(1, last + 1):
            yield Tick("ANVIL-1", level)
