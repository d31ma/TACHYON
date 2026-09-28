import 'dart:io';

import 'models/Settings.dart';

@RequestRoute("/settings")
@Public
class SettingsController {
  @GetRoute
  Settings getSettings() {
    final applicationJson = Platform.environment['YON_APPLICATION_JSON']!;
    final application = BehaviorJson.members(BehaviorJson.parse(applicationJson));
    final catalog = BehaviorJson.members(Behaviors.properties(CatalogSetup, application));
    final endpoint = BehaviorJson.members(Behaviors.values(Endpoint, application));
    final profile = endpoint['profile'] as String;
    final context = Behaviors.create(CatalogSetup,
        partition: 'public-catalog', signal: BehaviorSignal(), profiles: [profile], properties: application);
    context.dispose();
    return Settings(profile, catalog['pageSize'] as int);
  }
}
