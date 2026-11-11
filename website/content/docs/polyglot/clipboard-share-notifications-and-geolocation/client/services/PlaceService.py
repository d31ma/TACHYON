from typing import Annotated

@service
class PlaceService:
    geolocation: Annotated[Geolocation, needs]
    clipboard: Annotated[Clipboard, needs]
    place: str = ''
    problem: str = ''

    def locate(self) -> None:
        try:
            self.geolocation.current_position()
        except RuntimeError as refusal:
            self.problem = str(refusal)

    def copy(self) -> None:
        try:
            self.clipboard.write_text(self.place)
        except RuntimeError as refusal:
            self.problem = str(refusal)

    @on_event
    def found(self, found: PositionFound) -> None:
        self.place = f'{found.latitude}, {found.longitude}'

    @on_event
    def failed(self, failed: BrowserCallFailed) -> None:
        self.problem = failed.message
