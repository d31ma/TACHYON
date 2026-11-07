from typing import Annotated

@checked
class ProductInput:
    name: Annotated[str, required, not_empty, length({"min":2,"max":4})] = ''
    price: Annotated[int, min(0), max(100)] = 0

