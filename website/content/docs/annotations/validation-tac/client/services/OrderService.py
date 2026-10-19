from typing import Annotated
from models import OrderInput


@service
class OrderService:
    status: str = 'ready'

    def accept(self, order: Annotated[OrderInput, check]) -> dict[str, object]:
        return order
