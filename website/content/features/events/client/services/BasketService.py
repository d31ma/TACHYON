@service
class BasketService:
    count: int
    label: str
    _selected: str
    tabs: list[Tab]

    def __init__(self) -> None:
        self.count = 0
        self.label = ""
        self._selected = ""
        self.tabs = [Tab(id="a", name="A")]

    def increment(self) -> None:
        self.count += 1

    def select(self, id: str) -> None:
        self._selected = id
