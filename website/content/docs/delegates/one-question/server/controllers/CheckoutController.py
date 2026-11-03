from typing import Annotated
from models import Receipt


@request_route("/checkout")
@public
class CheckoutController:
    service: Annotated[CheckoutService, needs]

    @get_route
    def get_checkout(self) -> Receipt:
        total = self.service.total()
        return Receipt(total)
