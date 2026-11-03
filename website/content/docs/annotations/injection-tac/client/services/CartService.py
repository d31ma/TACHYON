from typing import Annotated

@service
@lifetime({"value":"prototype"})
class CartService:
    cart_prices_delegate: Annotated[CartPricesDelegate, needs]
    cart_delegate: Annotated[CartDelegate, needs]

    def total(self) -> int:
        subtotal = self.cart_prices_delegate.subtotal()
        return self.cart_delegate.settle(subtotal)
