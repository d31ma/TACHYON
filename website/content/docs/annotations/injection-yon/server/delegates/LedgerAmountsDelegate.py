@delegate
@lifetime('singleton')
class LedgerAmountsDelegate:
    def subtotal(self) -> int:
        return 40

    def surcharge(self) -> int:
        return 2
