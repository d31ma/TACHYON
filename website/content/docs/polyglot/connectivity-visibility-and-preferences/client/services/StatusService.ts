@Service
export default class StatusService {
  online: boolean = true;
  scheme: string = 'light';

  @OnEvent({"replay":true})
  connectivityChanged(changed: ConnectivityChanged): void {
    this.online = changed.online;
  }

  @OnEvent({"replay":true})
  preferencesChanged(changed: PreferencesChanged): void {
    this.scheme = changed.scheme;
  }
}
