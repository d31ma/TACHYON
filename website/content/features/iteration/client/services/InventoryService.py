@service
class InventoryService:
    products: list[Product]

    def __init__(self) -> None:
        self.products = [Product(name="Anvil"), Product(name="Rope")]
