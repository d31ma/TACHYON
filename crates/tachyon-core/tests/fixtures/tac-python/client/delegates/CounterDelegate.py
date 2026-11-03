from typing import Annotated

@delegate
class CounterDelegate:
    audit: Annotated[AuditDelegate, needs]

    @on_start
    def _start(self) -> None:
        self.audit.record_start()

    @on_stop
    def _stop(self) -> None:
        self.audit.record_stop()

    def amount(self) -> int:
        return 3
