@Service
export default class PlatformService {
  @Needs
  view;
  nativeHost = false;
  @OnStart
  initialize() { this.nativeHost = this.view.ownerDocument.body.dataset.platform === 'native'; }
}