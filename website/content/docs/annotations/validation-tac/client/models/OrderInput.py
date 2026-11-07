from typing import Annotated


@checked
class OrderInput:
    name: Annotated[str, not_empty, length({"min":1,"max":80})] = ''
    level: Annotated[int, required, min(1), max(9)] = 0
