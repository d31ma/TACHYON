@helper
@lifetime('singleton')
class LedgerAmountsHelper:
    def subtotal(self) -> int:
        return 40

    def surcharge(self) -> int:
        return 2
