@service
class BasketService:
    count: int = 0
    label: str = ""

    @on_start
    def start(self) -> None:
        self.count = 6
        self.label = "Python"

    def doubled(self) -> int:
        return self.count * 2
