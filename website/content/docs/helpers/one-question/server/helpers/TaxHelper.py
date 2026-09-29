from typing import Annotated

@helper
class TaxHelper:
    rates: Annotated[TaxRateHelper, needs]

    def add_tax(self, subtotal: int) -> int:
        tax = self.rates.rate()
        return subtotal + tax
