from typing import Annotated

@service
class LedgerService:
    repository: Annotated[LedgerStore, needs]

    def subtotal(self) -> int:
        return self.repository.subtotal()
