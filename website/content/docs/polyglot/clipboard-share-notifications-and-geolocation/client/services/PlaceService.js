@Service
export default class PlaceService {
  /** @type {TacGeolocation} */
  @Needs
  geolocation;
  /** @type {TacClipboard} */
  @Needs
  clipboard;
  place = '';
  problem = '';

  locate() {
    this.geolocation.currentPosition();
  }

  copy() {
    this.clipboard.writeText(this.place);
  }

  /** @param {PositionFound} found */
  @OnEvent
  found(found) {
    this.place = `${found.latitude}, ${found.longitude}`;
  }

  /** @param {BrowserCallFailed} failed */
  @OnEvent
  failed(failed) {
    this.problem = failed.message;
  }
}
