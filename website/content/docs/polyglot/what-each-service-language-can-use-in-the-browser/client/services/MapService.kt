import models.PlaceFound

@Service
class MapService {
    var latitude: Double = 0.0
    var longitude: Double = 0.0

    @OnEvent
    fun show(value: PlaceFound) {
        latitude = value.latitude
        longitude = value.longitude
    }
}
