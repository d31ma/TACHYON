from models import LedgerReport


@request_route("/ledger")
@public
class LedgerController:
    @get_route
    async def get_ledger(self) -> LedgerReport:
        context = await Behaviors.create(LedgerSetup, {'partition': 'ledger', 'signal': BehaviorSignal()})
        entries = Behaviors.construct(EntryStore, {'behaviors': context})
        settled = await entries.settle('open')
        context.dispose()
        return LedgerReport(settled, LedgerAdapterHelper.commands)
