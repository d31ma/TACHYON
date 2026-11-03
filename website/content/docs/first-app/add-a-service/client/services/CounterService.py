@service
class CounterService:
    count: int = 0

    def increment(self) -> None:
        self.count += 1
