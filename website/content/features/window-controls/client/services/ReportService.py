from collections.abc import Callable
from typing import Annotated


@service
class ReportService:
    window_controls: Annotated[WindowControls, needs]
    problem: str = ''

    def minimize(self) -> None:
        self._keep_problem(self.window_controls.minimize)

    def resize(self) -> None:
        self._keep_problem(lambda: self.window_controls.resize(1280, 800))

    def rename(self) -> None:
        self._keep_problem(lambda: self.window_controls.set_title('Quarterly report'))

    def _keep_problem(self, call: Callable[[], None]) -> None:
        try:
            call()
        except RuntimeError as refusal:
            self.problem = str(refusal)
