from typing import Annotated

@checked
class ProductInput:
    name: Annotated[str, not_empty, length({"min":1,"max":80})] = ''

    level: Annotated[int, required, min(1), max(9)] = 0
