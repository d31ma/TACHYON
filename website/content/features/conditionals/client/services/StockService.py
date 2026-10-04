@service
class StockService:
    products: list[Product]
    restocking: bool

    def __init__(self) -> None:
        self.products = []
        self.restocking = True
