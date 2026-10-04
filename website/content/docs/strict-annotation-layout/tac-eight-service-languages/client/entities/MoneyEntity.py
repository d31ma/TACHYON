from typing import Annotated


@entity({"adapter": "publicMoney"})
@table("money")
class MoneyEntity:
    id: Annotated[int, id] = 0
