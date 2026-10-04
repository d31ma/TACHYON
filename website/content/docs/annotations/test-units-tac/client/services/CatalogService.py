@service
class CatalogService:
    title: str = 'Catalog'

    def double(self, value: int) -> int:
        return value * 2
