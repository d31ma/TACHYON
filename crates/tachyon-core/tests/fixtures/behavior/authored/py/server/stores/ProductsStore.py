from typing import Annotated

@store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
class ProductsStore:
    behaviors: Annotated[object, needs]
    async def find_by_status(self, status):
        pass
    async def countByStatus(self, status):
        pass
    @query({"operation":"exists","where":[{"field":"status","operator":"eq","argument":0}],"orderBy":[]})
    async def available(self, status):
        pass

