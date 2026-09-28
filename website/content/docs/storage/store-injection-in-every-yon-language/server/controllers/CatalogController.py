from typing import Annotated

@request_route("/catalog")
@public
class CatalogController:
    catalog: Annotated[CatalogStore, needs]
    @get_route
    def get_catalog(self) -> list[dict[str, object]]:
        return self.catalog.find()