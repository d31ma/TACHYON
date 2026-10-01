@helper
class AuditHelper:
    start_count: int = 0
    stop_count: int = 0

    def record_start(self) -> None:
        self.start_count += 1

    def record_stop(self) -> None:
        self.stop_count += 1

    def snapshot(self) -> list[int]:
        return [self.start_count, self.stop_count]
