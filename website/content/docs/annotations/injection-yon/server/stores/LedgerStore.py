@store
@lifetime('singleton')
class LedgerStore:
    def subtotal(self) -> int:
        return 40

    def surcharge(self) -> int:
        return 2
