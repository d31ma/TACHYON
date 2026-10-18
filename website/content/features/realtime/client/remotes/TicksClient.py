from typing import Protocol

from models import Tick


@http_client("/ticks")
class TicksClient(Protocol):
    @stream_request
    def receive(self, tick: Tick) -> None: ...
