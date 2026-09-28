import time
from typing import Iterator

from models import Tick


@request_route("/ticks")
@public
class TicksController:
    @stream_route
    def stream_ticks(self) -> Iterator[Tick]:
        yield from self._countdown(3)

    def _countdown(self, start: int) -> Iterator[Tick]:
        for level in range(start, 0, -1):
            yield Tick("ANVIL-1", level)
            time.sleep(1)
