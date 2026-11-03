from typing import Annotated
from models import CatalogSummary


@request_route("/catalog")
@public
class CatalogController:
    converter: Annotated[ProductConverter, needs]

    @get_route
    async def get_catalog(self) -> CatalogSummary:
        product = {'detail': {'id': 7}, 'name': 'Anvil', 'extra': 'dropped'}
        summary = await self.converter.to_summary(product)
        return CatalogSummary(summary)
