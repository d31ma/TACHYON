@Service
export default class SettingsService {
  @Needs
  readonly applicationClient!: ApplicationClient;
  api = '';
  async load() {
    const application = await this.applicationClient.load() as BehaviorJsonObject;
    const settings = Behaviors.values(PublicSettings, application);
    this.api = String(settings.api);
  }
}
