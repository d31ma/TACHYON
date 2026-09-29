@helper
class BillingHelper:
    _SURCHARGE = 2

    def settle(self, subtotal: int) -> int:
        return subtotal + self._SURCHARGE
