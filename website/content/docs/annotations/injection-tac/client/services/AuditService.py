from typing import Annotated

@service
class AuditService:
    ledger: Annotated[CartDelegate, needs, named("CartDelegate")]

    def lines(self) -> int:
        return self.ledger.count()
