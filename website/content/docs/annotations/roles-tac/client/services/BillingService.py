from typing import Annotated


@service
class BillingService:
    billing_delegate: Annotated[BillingDelegate, needs]
    total: int = 0
    _SUBTOTAL = 40

    @on_start
    def open(self) -> None:
        self.total = self.billing_delegate.settle(self._SUBTOTAL)
