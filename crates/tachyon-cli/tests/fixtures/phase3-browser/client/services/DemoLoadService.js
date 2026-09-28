@Service
@Lifetime({"value":"prototype"})
export default class DemoLoadService {
  @Needs
  view
  label = ''
  constructor(props) {
    this.label = props.label
  }

  @OnStart
  initialize() {
    this.view.dataset.activated = this.label
  }
}
