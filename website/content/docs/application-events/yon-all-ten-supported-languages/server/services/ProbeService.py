from typing import Optional
from models import ProbeReady


@service
class ProbeService:
    last: Optional[ProbeReady] = None

    @on_event
    def observe(self, ready: ProbeReady) -> None:
        self.last = ready

    @on_stop
    def destroy(self) -> None:
        self.last = None
