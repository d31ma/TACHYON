@request_route("/ticks")
class TicksController:
    @stream_route
    def stream_ticks(self):
        yield from self._ticks_up_to(3)

    def _ticks_up_to(self, last):
        for tick in range(1, last + 1):
            yield {"tick": tick}