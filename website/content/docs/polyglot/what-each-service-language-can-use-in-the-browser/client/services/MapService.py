from models import PlaceFound


@service
class MapService:
    latitude: float = 0.0
    longitude: float = 0.0

    @on_event
    def show(self, value: PlaceFound) -> None:
        self.latitude = value.latitude
        self.longitude = value.longitude
