from typing import Annotated

@delegate
class TaxDelegate:
    tax_service: Annotated[TaxService, needs]

    def rate(self, region: str) -> int:
        return self.tax_service.rate(region) * 2

    def total(self, count: int, extra: int) -> int:
        return self.tax_service.total(count) + extra
