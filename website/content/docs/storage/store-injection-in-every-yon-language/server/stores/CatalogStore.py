from typing import Annotated

@store
class CatalogStore:
    data_source: Annotated[DataSource, needs]
    def find(self) -> list[dict[str, object]]:
        return self.data_source.query('SELECT 42 AS amount', [])