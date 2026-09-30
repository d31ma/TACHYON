@Service
export default class PlatformService {
  @Needs
  view!: HTMLElement;
  nativeHost: boolean = false;
  @OnStart
  initialize(): void { this.nativeHost = this.view.ownerDocument.body.dataset.platform === 'native'; }
}
