@Service
@Lifetime({"value":"prototype"})
export default class DemoFailureService {
  label = ''
  constructor(props) {
    this.label = props.label
  }

  @OnStart
  initialize() {
    throw new Error('expected activation failure')
  }
}
