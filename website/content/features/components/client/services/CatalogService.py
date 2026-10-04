@service
class CatalogService:
    products: list[Product]

    def __init__(self) -> None:
        self.products = [
            Product(id="anvil", name="Anvil", price="£42.00"),
            Product(id="rope", name="Rope", price="£8.00"),
        ]
