from typing import Annotated

@store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
class EntryStore:
    behaviors: Annotated[object, needs]
    async def find_by_status(self, status: str) -> None:
        pass
    async def count_by_status(self, status: str) -> None:
        pass

    @all_or_nothing({"adapter":"ledger"})
    async def settle(self, status: str) -> dict[str, object]:
        open_entries = await self.find_by_status(status)
        total = await self.count_by_status(status)
        return {'open': open_entries, 'total': total}
