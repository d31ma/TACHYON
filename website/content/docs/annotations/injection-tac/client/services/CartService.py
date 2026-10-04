from typing import Annotated

@service
@lifetime({"value":"prototype"})
class CartService:
    cart_prices_helper: Annotated[CartPricesHelper, needs]
    cart_helper: Annotated[CartHelper, needs]

    def total(self) -> int:
        subtotal = self.cart_prices_helper.subtotal()
        return self.cart_helper.settle(subtotal)
