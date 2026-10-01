from typing import ClassVar

@service
class ShowcaseService:
    _TAC_SUMMARY: ClassVar[str] = "One component runs on every target. The view and the stylesheet are the same everywhere. Pick a platform to see every language that can write the service there."
    _YON_SUMMARY: ClassVar[str] = "In all eleven languages, a controller uses the same route annotations. Services, stores and other application modules stay ordinary code. The language itself puts them together."

    layer: str = ""

    @on_start
    def _start(self) -> None:
        self.layer = "tac"

    def pick_layer(self, value: str) -> None:
        self.layer = value

    def summary(self) -> str:
        return self._summary_for(self.layer)

    def _summary_for(self, layer: str) -> str:
        if layer == "tac":
            return ShowcaseService._TAC_SUMMARY
        return ShowcaseService._YON_SUMMARY
