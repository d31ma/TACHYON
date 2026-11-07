from typing import Annotated

@entity({"adapter":"billing"})
@table("invoices")
class BillingEntity:
    id: Annotated[int, id()] = 0
