@Service
@Lifetime({"value":"prototype"})
export default class DemoInteractiveService {
  @Needs
  view
  @Needs
  lifecycle
  label = ''
  constructor(props) {
    this.label = props.label
  }

  @OnStart
  initialize() {
    const root = this.view
    root.dataset.activated = this.label
    root.addEventListener('pointerdown', () => this.#countReplay(root), { signal: this.lifecycle })
  }

  #countReplay(root) {
    const replayCount = Number(root.dataset.replayed || '0') + 1
    root.dataset.replayed = String(replayCount)
  }
}
