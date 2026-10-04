from typing import Annotated

@helper
class CartHelper:
    cart_prices_helper: Annotated[CartPricesHelper, needs]

    def settle(self, subtotal: int) -> int:
        surcharge = self.cart_prices_helper.surcharge()
        return subtotal + surcharge

    def count(self) -> int:
        return self.cart_prices_helper.count()
