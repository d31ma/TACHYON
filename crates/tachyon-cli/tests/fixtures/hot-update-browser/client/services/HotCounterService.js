@Service
export default class HotCounterService {
  static _FLAG_SET = 'true'
  count = 1
  version = 'one'
  /** @type {HTMLElement} */
  @Needs
  view
  /** @type {AbortSignal} */
  @Needs
  lifecycle

  @OnStart
  initialize() {
    this.view.dataset.moduleVersion = this.version
    this.lifecycle.addEventListener('abort', () => {
      sessionStorage.setItem('tachyon-hot-aborted', HotCounterService._FLAG_SET)
    }, { once: true })
  }

  increment() {
    this.count += 1
  }

  /** @returns {{ count: number }} */
  hotState() {
    return { count: this.count }
  }

  /**
   * @param {{ count: number }} state
   */
  restoreHotState(state) {
    this.count = state.count
  }

  hotDispose() {
    sessionStorage.setItem('tachyon-hot-disposed', HotCounterService._FLAG_SET)
  }
}
