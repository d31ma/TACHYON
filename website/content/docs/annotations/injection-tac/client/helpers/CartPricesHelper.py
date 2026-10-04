@helper
class CartPricesHelper:
    def subtotal(self) -> int:
        return 40

    def surcharge(self) -> int:
        return 2

    def count(self) -> int:
        return 3
