// @ts-check

@Service
export default class ShellService {
  /** @type {string} */
  headline = 'Tachyon full-stack fixture'

  @OnStart
  initialize() {
    if (typeof document !== 'undefined') {
      document.title = this.headline
    }
  }
}
