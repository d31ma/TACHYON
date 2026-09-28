@Service
export default class PlaceService {
  /** @type {ApplicationEventPublisher} */
  applicationEventPublisher;

  @OnStart
  locate() {
    navigator.geolocation.getCurrentPosition((position) => this._found(position));
  }

  /** @param {GeolocationPosition} position */
  _found(position) {
    /** @type {PlaceFound} */
    const place = { latitude: position.coords.latitude, longitude: position.coords.longitude };
    this.applicationEventPublisher.publishEvent(place, false);
  }
}
