from typing import Annotated
from models import Product


@request_route("/products")
@public
class ProductsController:
    catalog: Annotated[ProductsService, needs, named("ProductsService")]

    @get_route
    def get_products(self) -> list[Product]:
        return self.catalog.list()

    @get_route("/{id}")
    def get_product(self, id: Annotated[int, from_path]) -> Product:
        return self.catalog.find(id)
