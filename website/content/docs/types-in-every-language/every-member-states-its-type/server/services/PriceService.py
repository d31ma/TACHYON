from models import Price


@service
class PriceService:
    _CURRENCY = "EUR"

    def price_for(self, amount: int, count: int) -> Price:
        return Price(amount * count, PriceService._CURRENCY)
