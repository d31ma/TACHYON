@service
class TallyService:
    count: int = 0

    def add(self, amount: int) -> int:
        self.count += amount
        return self.count

    def summary(self, unit: str) -> str:
        return f"{self.count} {unit}"
