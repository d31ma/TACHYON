from models import Order


@Service
class OrderService:
    def __init__(self):
        self.repository = OrderStore()

    def list(self) -> list[Order]:
        return self.repository.all()
