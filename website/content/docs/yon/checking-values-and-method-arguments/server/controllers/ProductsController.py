from typing import Annotated
from dataclasses import asdict
from models import Product


@request_route("/products")
@public
class ProductsController:
    product_service: Annotated[ProductService, needs]

    @post_route
    def post_product(self, product: Annotated[Product, from_body]) -> Product:
        return self.product_service.accept(asdict(product))
