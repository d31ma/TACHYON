import 'models/PublicSettings.dart';

@Service
class SettingsService {
  @Needs
  final ApplicationClient applicationClient;
  String api = '';

  Future<void> load() async {
    final application = await applicationClient.load();
    final settings = BehaviorJson.members(Behaviors.values(PublicSettings, application));
    api = settings['api'] as String;
  }
}
