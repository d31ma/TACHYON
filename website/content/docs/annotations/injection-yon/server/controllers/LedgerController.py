from typing import Annotated
from models import Ledger


@request_route("/ledger")
@public
class LedgerController:
    service: Annotated[LedgerService, needs, named("LedgerService")]
    settlement: Annotated[LedgerDelegate, needs, named("LedgerDelegate")]

    @get_route
    def get_ledger(self) -> Ledger:
        subtotal = self.service.subtotal()
        total = self.settlement.settle(subtotal)
        return Ledger(total)
