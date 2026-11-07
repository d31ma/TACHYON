@Service
export default class PlatformService {
  /** @type {HTMLElement} */
  @Needs
  view;
  /** @type {boolean} */
  nativeHost = false;
  @OnStart
  initialize() { this.nativeHost = this.view.ownerDocument.body.dataset.platform === 'native'; }
}
