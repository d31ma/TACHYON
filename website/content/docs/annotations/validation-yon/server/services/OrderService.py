from typing import Annotated
from models import OrderInput

@service
class OrderService:
    def accept(self, order: Annotated[OrderInput, check]) -> OrderInput:
        return order
