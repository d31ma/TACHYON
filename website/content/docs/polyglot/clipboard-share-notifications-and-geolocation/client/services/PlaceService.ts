@Service
export default class PlaceService {
  @Needs
  geolocation!: TacGeolocation;
  @Needs
  clipboard!: TacClipboard;
  place: string = '';
  problem: string = '';

  locate(): void {
    this.geolocation.currentPosition();
  }

  copy(): void {
    this.clipboard.writeText(this.place);
  }

  @OnEvent
  found(found: PositionFound): void {
    this.place = `${found.latitude}, ${found.longitude}`;
  }

  @OnEvent
  failed(failed: BrowserCallFailed): void {
    this.problem = failed.message;
  }
}
