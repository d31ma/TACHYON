from typing import Protocol

@http_client("/billing")
class BillingClient(Protocol):
    @get_request("/surcharge")
    async def surcharge(self) -> object: ...
