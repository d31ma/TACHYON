from typing import Annotated


@service
class LedgerService:
    behaviors: object = None
    entries: Annotated[EntryStore, needs]

    @on_start
    async def open(self) -> None:
        self.behaviors = await Behaviors.create(LedgerSetup, {'partition': 'ledger', 'signal': BehaviorSignal()})

    @all_or_nothing({"adapter":"ledger"})
    async def settle(self, status: str) -> dict[str, object]:
        open_entries = await self.entries.find_by_status(status)
        total = await self.entries.count_by_status(status)
        return {'open': open_entries, 'total': total}
