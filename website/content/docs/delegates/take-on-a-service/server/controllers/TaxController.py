from typing import Annotated
from models import TaxAnswer


@request_route("/tax")
@public
class TaxController:
    _COUNT = 3

    tax_delegate: Annotated[TaxDelegate, needs]

    @get_route
    def get_tax(self) -> TaxAnswer:
        return TaxAnswer(
            rate=self.tax_delegate.rate("north"),
            total=self.tax_delegate.total(self._COUNT),
            extra=self.tax_delegate.total(self._COUNT, 4),
        )
