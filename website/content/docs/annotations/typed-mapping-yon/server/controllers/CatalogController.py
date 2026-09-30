from typing import Annotated
from models import Product, Detail, Summary
@RequestRoute("/catalog")
@Public
class CatalogController:
    converter: Annotated[ProductConverter, Needs]
    @GetRoute
    async def get_catalog(self) -> Summary:
        detail = Detail('detail', 'hidden')
        return self.converter.map(Product(7, 'Anvil', detail, [detail], 'hidden'))
