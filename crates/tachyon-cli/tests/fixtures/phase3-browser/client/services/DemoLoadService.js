@Service
@Lifetime({"value":"prototype"})
export default class DemoLoadService {
  /** @type {HTMLElement} */
  @Needs
  view
  label = ''
  /**
   * @param {{ label: string }} props
   */
  constructor(props) {
    this.label = props.label
  }

  @OnStart
  initialize() {
    this.view.dataset.activated = this.label
  }
}
