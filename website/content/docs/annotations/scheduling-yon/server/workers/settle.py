from typing import Annotated


@schedule({"fixedDelaySeconds":300})
class Handler:
    ledger: Annotated[LedgerService, needs]

    def POST(self) -> int:
        return self.ledger.total()
