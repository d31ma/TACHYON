from typing import Annotated
from models import LedgerReport


@request_route("/ledger")
@public
class LedgerController:
    ledger_service: Annotated[LedgerService, needs]

    @get_route
    async def get_ledger(self) -> LedgerReport:
        settled = await self.ledger_service.settle('open')
        return LedgerReport(settled, LedgerAdapterDelegate.commands)
