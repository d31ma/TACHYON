@service
class TallyService:
    count: int = 6
    runtime: str = "Python, on the pinned Pyodide web runtime"

    def doubled(self) -> int:
        return self.count * 2
