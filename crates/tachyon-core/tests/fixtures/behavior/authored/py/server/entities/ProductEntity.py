from typing import Annotated

@entity({"adapter":"data"})
@table("products")
class ProductEntity:
    id: Annotated[int, id()] = 0

