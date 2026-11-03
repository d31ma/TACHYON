@Service
final class MapService {
    var latitude: Double = 0
    var longitude: Double = 0

    @OnEvent
    func show(_ value: PlaceFound) {
        latitude = value.latitude
        longitude = value.longitude
    }
}
