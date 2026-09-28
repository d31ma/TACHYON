from typing import Annotated

@helper
class BillingHelper:
    client: Annotated[BillingClient, needs]

    def settle(self, subtotal: int) -> int:
        surcharge = self.client.surcharge()
        return subtotal + surcharge
