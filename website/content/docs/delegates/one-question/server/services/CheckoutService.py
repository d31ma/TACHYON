from typing import Annotated

@service
class CheckoutService:
    helper: Annotated[TaxDelegate, needs]

    def total(self) -> int:
        subtotal = self._subtotal()
        return self.helper.add_tax(subtotal)

    def _subtotal(self) -> int:
        return 40
