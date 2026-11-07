import type { PlaceFound } from '/models/PlaceFound.ts';

@Service
export default class MapService {
  latitude: number = 0;
  longitude: number = 0;

  @OnEvent
  show(value: PlaceFound): void {
    this.latitude = value.latitude;
    this.longitude = value.longitude;
  }
}
