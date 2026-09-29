@helper
class BillingHelper:
    SURCHARGE = 2

    def settle(self, subtotal: int) -> int:
        return subtotal + self.SURCHARGE
