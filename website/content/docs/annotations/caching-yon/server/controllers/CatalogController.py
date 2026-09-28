from typing import Annotated
from models import CacheReport


@request_route("/catalog")
@public
class CatalogController:
    catalog: Annotated[CatalogService, needs]

    @get_route
    async def get_catalog(self) -> CacheReport:
        sku = 'anvil'
        first = await self.catalog.find(sku)
        repeat = await self.catalog.find(sku)
        await self.catalog.write(sku, 99)
        written = await self.catalog.find(sku)
        await self.catalog.invalidate(sku)
        refreshed = await self.catalog.find(sku)
        return CacheReport(first, repeat, written, refreshed)
