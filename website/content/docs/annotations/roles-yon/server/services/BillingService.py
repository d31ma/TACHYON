from typing import Annotated

@service
class BillingService:
    _SUBTOTAL: int = 40
    helper: Annotated[BillingDelegate, needs]

    def total(self) -> int:
        return self.helper.settle(self._SUBTOTAL)
