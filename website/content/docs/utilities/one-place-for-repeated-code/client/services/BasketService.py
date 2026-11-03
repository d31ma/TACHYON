@service
class BasketService:
    _total: int = 1250

    def total_label(self) -> str:
        return PriceUtils.label(self._total)
