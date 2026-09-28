from typing import Annotated
from models import Lifecycle


@request_route("/lifecycle")
@public
class LifecycleController:
    audit: Annotated[AuditService, needs]
    checkout: Annotated[CheckoutService, needs]

    @get_route
    def get_lifecycle(self) -> Lifecycle:
        return Lifecycle(self.audit.log, self.audit.settled, self.audit.audited)
