from typing import Annotated

@delegate
class TaxDelegate:
    rates: Annotated[TaxRateDelegate, needs]

    def add_tax(self, subtotal: int) -> int:
        tax = self.rates.rate()
        return subtotal + tax
