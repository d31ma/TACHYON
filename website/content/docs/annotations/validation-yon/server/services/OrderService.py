from typing import Annotated

@service
class OrderService:
    def accept(self, order: Annotated[OrderInput, check]) -> OrderInput:
        return order
