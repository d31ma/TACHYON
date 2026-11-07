@Service
export default class StatusService {
  online = true;
  scheme = 'light';

  /** @param {ConnectivityChanged} changed */
  @OnEvent({"replay":true})
  connectivityChanged(changed) {
    this.online = changed.online;
  }

  /** @param {PreferencesChanged} changed */
  @OnEvent({"replay":true})
  preferencesChanged(changed) {
    this.scheme = changed.scheme;
  }
}
