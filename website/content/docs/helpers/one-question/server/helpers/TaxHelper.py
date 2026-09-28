from typing import Annotated

@helper
class TaxHelper:
    store: Annotated[TaxRateStore, needs]

    def add_tax(self, subtotal: int) -> int:
        tax = self.store.rate()
        return subtotal + tax
