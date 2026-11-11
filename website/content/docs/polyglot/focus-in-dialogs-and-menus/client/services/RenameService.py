from collections.abc import Callable
from typing import Annotated


@service
class RenameService:
    focus: Annotated[Focus, needs]
    editing: bool = False
    problem: str = ''

    def open(self) -> None:
        self.editing = True
        self._keep_problem(lambda: self.focus.trap('dialog'))
        self._keep_problem(lambda: self.focus.move('name'))

    def close(self) -> None:
        self.editing = False
        self._keep_problem(self.focus.release)

    @on_event
    def failed(self, failed: BrowserCallFailed) -> None:
        self.problem = failed.message

    def _keep_problem(self, call: Callable[[], None]) -> None:
        try:
            call()
        except RuntimeError as refusal:
            self.problem = str(refusal)
