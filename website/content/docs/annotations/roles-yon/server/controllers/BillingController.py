from typing import Annotated
from models import Bill


@request_route("/billing")
@public
class BillingController:
    service: Annotated[BillingService, needs]

    @get_route
    def get_billing(self) -> Bill:
        total = self.service.total()
        return Bill(total)
