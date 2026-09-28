@Service
@Lifetime({"value":"prototype"})
export default class DemoInteractiveService {
  /** @type {HTMLElement} */
  @Needs
  view
  /** @type {AbortSignal} */
  @Needs
  lifecycle
  label = ''
  /**
   * @param {{ label: string }} props
   */
  constructor(props) {
    this.label = props.label
  }

  @OnStart
  initialize() {
    const root = this.view
    root.dataset.activated = this.label
    root.addEventListener('pointerdown', () => this.#countReplay(root), { signal: this.lifecycle })
  }

  /**
   * @param {HTMLElement} root
   */
  #countReplay(root) {
    const replayCount = Number(root.dataset.replayed || '0') + 1
    root.dataset.replayed = String(replayCount)
  }
}
