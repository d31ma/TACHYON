from typing import Annotated

@request_route("/catalog")
@public
class CatalogController:
    catalog: Annotated[CatalogClient, needs]

    @get_route
    async def get_catalog(self) -> dict[str, object]:
        return {
            'found': await self.catalog.find("anvil", "full", "acme"),
            'saved': await self.catalog.save({'sku': "anvil", 'level': 3}),
            'replaced': await self.catalog.replace(5, {'sku': "anvil", 'level': 4}),
            'renamed': await self.catalog.rename(6, {'sku': "hammer", 'level': 1}),
            'removed': await self.catalog.remove(7),
        }
