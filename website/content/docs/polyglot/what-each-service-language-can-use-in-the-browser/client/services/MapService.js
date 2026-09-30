/** @import { PlaceFound } from '/models/PlaceFound.js' */
@Service
export default class MapService {
  latitude = 0;
  longitude = 0;

  /** @param {PlaceFound} value */
  @OnEvent
  show(value) {
    this.latitude = value.latitude;
    this.longitude = value.longitude;
  }
}
