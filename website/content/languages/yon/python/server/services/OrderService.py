from models import Order


@Service
class OrderService:
    def list(self) -> list[Order]:
        return [Order("anvil", 4)]
