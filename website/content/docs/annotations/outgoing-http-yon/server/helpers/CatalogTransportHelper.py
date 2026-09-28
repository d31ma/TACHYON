@Helper
class CatalogTransportHelper:
    @staticmethod
    async def exchange(request: object, execution: object) -> dict[str, list[str]]:
        return {'items': ['anvil']}
