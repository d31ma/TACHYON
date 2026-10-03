from typing import Annotated
from models import Product, Detail, Summary


@request_route("/catalog")
@public
class CatalogController:
    converter: Annotated[ProductConverter, needs]

    @get_route
    async def get_catalog(self) -> Summary:
        detail = Detail('detail', 'detail-private')
        return self.converter.map(Product(7, 'Anvil', detail, [detail], 'product-private'))
