from typing import Annotated

@delegate
class CartDelegate:
    cart_prices_delegate: Annotated[CartPricesDelegate, needs]

    def settle(self, subtotal: int) -> int:
        surcharge = self.cart_prices_delegate.surcharge()
        return subtotal + surcharge

    def count(self) -> int:
        return self.cart_prices_delegate.count()
