from typing import Annotated

@service
class CatalogService:
    catalog: Annotated[CatalogClient, needs]
    async def products(self) -> None:
        return await self.catalog.all(1)
