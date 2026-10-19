@Service
export default class ActivityService {
  state: string = 'foreground';

  @OnEvent({"replay":true})
  applicationStateChanged(changed: ApplicationStateChanged): void {
    this.state = changed.state;
  }
}
