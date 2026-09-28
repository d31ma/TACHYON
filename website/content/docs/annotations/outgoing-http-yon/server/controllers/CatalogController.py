from typing import Annotated

@request_route("/catalog")
@public
class CatalogController:
    catalog: Annotated[CatalogService, needs]
    @get_route
    async def get_catalog(self) -> None:
        return await self.catalog.products()
