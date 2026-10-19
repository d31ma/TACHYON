@Service
export default class ActivityService {
  state = 'foreground';

  /** @param {ApplicationStateChanged} changed */
  @OnEvent({"replay":true})
  applicationStateChanged(changed) {
    this.state = changed.state;
  }
}
