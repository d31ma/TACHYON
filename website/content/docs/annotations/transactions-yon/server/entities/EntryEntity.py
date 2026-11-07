from typing import Annotated

@entity({"adapter":"ledger"})
@table("entries")
class EntryEntity:
    id: Annotated[int, id] = 0
