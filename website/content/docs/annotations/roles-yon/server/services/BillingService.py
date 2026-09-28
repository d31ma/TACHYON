from typing import Annotated

@service
class BillingService:
    repository: Annotated[BillingStore, needs]
    helper: Annotated[BillingHelper, needs]

    def total(self) -> int:
        subtotal = self.repository.subtotal()
        return self.helper.settle(subtotal)
