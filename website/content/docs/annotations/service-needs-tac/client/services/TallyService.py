from typing import Annotated

@service
class TallyService:
    counter: Annotated[CounterService, needs]
    total: int = 0

    def bump(self) -> None:
        self.total = self.counter.increment()
