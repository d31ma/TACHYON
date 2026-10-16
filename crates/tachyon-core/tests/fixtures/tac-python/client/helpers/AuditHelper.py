@helper
class AuditHelper:
    _start_count: int = 0
    _stop_count: int = 0

    def record_start(self) -> None:
        self._start_count += 1

    def record_stop(self) -> None:
        self._stop_count += 1

    def snapshot(self) -> list[int]:
        return [self._start_count, self._stop_count]
