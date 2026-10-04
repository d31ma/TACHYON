@service
class BasketService:
    count: int = 0

    def doubled(self) -> int:
        return self.count * 2

    @on_start
    def initialize(self) -> None:
        self.count = 6
