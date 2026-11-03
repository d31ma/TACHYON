@service
class TaxService:
    def rate(self, region: str) -> int:
        return 2

    def total(self, count: int) -> int:
        return count * 10
