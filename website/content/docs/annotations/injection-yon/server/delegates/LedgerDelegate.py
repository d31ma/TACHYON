from typing import Annotated

@delegate
@lifetime('transient')
class LedgerDelegate:
    amounts: Annotated[LedgerAmountsDelegate, needs]

    def settle(self, subtotal: int) -> int:
        surcharge = self.amounts.surcharge()
        return subtotal + surcharge
