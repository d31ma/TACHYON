from typing import Annotated

@service
class BillingService:
    _SUBTOTAL: int = 40
    helper: Annotated[BillingHelper, needs]

    def total(self) -> int:
        return self.helper.settle(self._SUBTOTAL)
