import 'models/PlaceFound.dart';

@Service
class MapService {
  double latitude = 0;
  double longitude = 0;

  @OnEvent
  void show(PlaceFound value) {
    latitude = value.latitude;
    longitude = value.longitude;
  }
}
