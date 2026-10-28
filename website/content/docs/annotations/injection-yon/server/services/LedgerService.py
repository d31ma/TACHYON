from typing import Annotated

@service
class LedgerService:
    amounts: Annotated[LedgerAmountsDelegate, needs]

    def subtotal(self) -> int:
        return self.amounts.subtotal()
