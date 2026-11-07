from typing import Annotated
from models import ProductReport


@request_route("/products")
@public
class ProductController:
    products: Annotated[ProductStore, needs]

    @get_route
    async def get_product(self) -> ProductReport:
        active_status = 'active'
        active = await self.products.find_by_status(active_status)
        active_count = await self.products.count_by_status(active_status)
        others = await self.products.excluding(active_status, {'size': 5})
        first = await self.products.find_by_id(1)
        return ProductReport(active, active_count, others, first)
