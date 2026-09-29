from typing import Annotated

@service
class LedgerService:
    amounts: Annotated[LedgerAmountsHelper, needs]

    def subtotal(self) -> int:
        return self.amounts.subtotal()
