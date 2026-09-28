from models import Product


@service
@lifetime('transient')
class ProductsService:
    def list(self) -> list[Product]:
        return [self.find(1)]

    def find(self, id: int) -> Product:
        return Product(id, "Anvil")
