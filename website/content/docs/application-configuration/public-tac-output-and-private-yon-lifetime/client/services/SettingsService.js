@Service
export default class SettingsService {
  /** @type {ApplicationClient} */
  @Needs
  applicationClient;
  api = '/api';
  @OnStart
  async initialize() {
    const application = /** @type {BehaviorJsonObject} */ (await this.applicationClient.load());
    const settings = Behaviors.values(PublicSettings, application);
    this.api = String(settings.api);
  }
}
