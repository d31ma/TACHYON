@service
class CounterService:
    count: int = 0
    theme: str = "light"

    def increment(self) -> None:
        self.count += 1
