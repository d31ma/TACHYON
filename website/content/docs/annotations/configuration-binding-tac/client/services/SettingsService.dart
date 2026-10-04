import 'models/PublicValues.dart';

@Service
class SettingsService {
  String title = 'Catalog';
  int pageSize = 20;

  @OnStart
  void initialize() {
    final published = _publicManifest();
    final catalog = BehaviorJson.members(Behaviors.properties(CatalogSetup, published));
    final values = BehaviorJson.members(Behaviors.values(PublicValues, published));
    final context = Behaviors.create(CatalogSetup,
        partition: 'public-catalog', signal: BehaviorSignal(), profiles: ['public'], properties: published);
    context.dispose();
    title = values['title'] as String;
    pageSize = (catalog['pageSize'] as num).toInt();
  }

  Map<String, Object?> _publicManifest() {
    return {
      'tac': {
        'catalog': {
          'title': 'Catalog',
          'page': {'size': 20},
          'cache': {'enabled': true},
        },
      },
    };
  }
}
