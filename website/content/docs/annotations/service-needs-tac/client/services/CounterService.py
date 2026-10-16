@service
class CounterService:
    count: int = 0

    def increment(self) -> int:
        self.count += 1
        return self.count
