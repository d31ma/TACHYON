from typing import Annotated

@store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
class ProductStore:
    behaviors: Annotated[object, needs]
    async def find_by_status(self, status: str) -> None:
        pass
    async def count_by_status(self, status: str) -> None:
        pass
    @query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
    async def excluding(self, status: str, page: Annotated[object, page_defaults]) -> None:
        pass
    def bounded(self, request: Annotated[object, page_defaults({"defaultSize":20,"maxSize":100,"sort":["id"]})]) -> object:
        return request
