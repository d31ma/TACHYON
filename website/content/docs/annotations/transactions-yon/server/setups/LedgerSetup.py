@setup
class LedgerSetup:
    @staticmethod
    def configure(execution: object) -> dict[str, object]:
        return {'entities': {'ledger': LedgerAdapterDelegate}, 'authorizeEntity': LedgerSetup.authorize}

    @staticmethod
    async def authorize(command: dict[str, object], execution: object) -> bool:
        return True
