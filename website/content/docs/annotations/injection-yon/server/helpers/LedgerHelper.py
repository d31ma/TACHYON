from typing import Annotated

@helper
@lifetime('transient')
class LedgerHelper:
    amounts: Annotated[LedgerAmountsHelper, needs]

    def settle(self, subtotal: int) -> int:
        surcharge = self.amounts.surcharge()
        return subtotal + surcharge
