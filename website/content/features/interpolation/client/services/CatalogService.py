@service
class CatalogService:
    title: str
    count: int
    product: Product

    def __init__(self) -> None:
        self.title = "Catalog"
        self.count = 6
        self.product = Product(name="Anvil", price="£42.00")
