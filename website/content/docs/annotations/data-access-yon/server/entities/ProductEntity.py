from typing import Annotated

@entity({"adapter":"catalog"})
@table("products")
class ProductEntity:
    id: Annotated[int, id] = 0
