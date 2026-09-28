from models import Order


@Store
class OrderStore:
    def all(self) -> list[Order]:
        return [Order("anvil", 4)]
