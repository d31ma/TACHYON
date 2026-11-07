@service
class CardService:
    name: str = "Anvil"
    status: str = "waiting"

    @on_start
    def activate(self) -> None:
        self.status = "active"
