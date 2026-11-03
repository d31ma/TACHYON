from typing import Annotated

@service
class CatalogService:
    data_source: Annotated[DataSource, needs]
    def find(self) -> list[dict[str, object]]:
        return self.data_source.query('SELECT 42 AS amount', [])