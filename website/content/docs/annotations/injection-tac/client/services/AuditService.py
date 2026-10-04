from typing import Annotated

@service
class AuditService:
    ledger: Annotated[CartHelper, needs, named("CartHelper")]

    def lines(self) -> int:
        return self.ledger.count()
