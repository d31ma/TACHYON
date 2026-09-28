from models import ProbeReady


@service
class ProbeService:
    last: ProbeReady | None = None

    @on_event
    def observe(self, ready: ProbeReady) -> None:
        self.last = ready

    @on_stop
    def destroy(self) -> None:
        self.last = None
