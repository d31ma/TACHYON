@service
class LedgerService:
    def balance(self, account: str) -> dict[str, object]:
        return {"account": account, "balance": 1250.5, "language": "python"}
