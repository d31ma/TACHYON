from typing import Annotated

@helper
@lifetime('transient')
class LedgerHelper:
    repository: Annotated[LedgerStore, needs]

    def settle(self, subtotal: int) -> int:
        surcharge = self.repository.surcharge()
        return subtotal + surcharge
