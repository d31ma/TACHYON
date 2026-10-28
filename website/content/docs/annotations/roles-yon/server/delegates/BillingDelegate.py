@delegate
class BillingDelegate:
    _SURCHARGE = 2

    def settle(self, subtotal: int) -> int:
        return subtotal + self._SURCHARGE
