@delegate
class BillingDelegate:
    def settle(self, subtotal: int) -> int:
        return subtotal + 2
