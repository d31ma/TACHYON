@service
class RuntimeNoteService:
    runtime: str = "Python, on the pinned Pyodide web runtime"

    def shows_notes(self, development: bool) -> bool:
        return development
