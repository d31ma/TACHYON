@service
class TickerService:
    seconds: int = 0

    @every("1s")
    def count(self) -> None:
        self.seconds += 1
