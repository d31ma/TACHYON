from typing import Annotated
from models import Product


@request_route("/products")
@public
class ProductsController:
    @get_route
    def get_products(self) -> list[Product]:
        return [Product("anvil")]

    @post_route
    @response_status(201)
    def post_product(self, product: Annotated[Product, from_body]) -> Product:
        return product
