import 'dart:convert';
import 'dart:io';

import 'models/Configuration.dart';

@RequestRoute("/configuration")
@Public
class ConfigurationController {
  @GetRoute
  Configuration getConfiguration() {
    final applicationJson = Platform.environment['YON_APPLICATION_JSON']!;
    final parsed = jsonDecode(applicationJson) as Map;
    final application = parsed.cast<String, Object?>();
    final settings = Behaviors.properties(CatalogSettings, application) as Map;
    final endpoint = Behaviors.values(Endpoint, application) as Map;
    return Configuration(settings['pageSize'] as int, endpoint['port'] as int);
  }
}
