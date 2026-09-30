import type { PlaceFound } from '/models/PlaceFound.ts';

@Service
export default class PlaceService {
  applicationEventPublisher!: ApplicationEventPublisher;

  @OnStart
  locate(): void {
    navigator.geolocation.getCurrentPosition((position) => this.found(position));
  }

  private found(position: GeolocationPosition): void {
    const place: PlaceFound = { latitude: position.coords.latitude, longitude: position.coords.longitude };
    this.applicationEventPublisher.publishEvent(place, false);
  }
}
