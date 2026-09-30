@Service
class CatalogService {
  late BehaviorContext behaviors;
  int offset = 0;

  Future<Object?> page(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) Map<String, Object?> input) async {
    return input['offset'];
  }

  @OnStart
  Future<void> initialize() async {
    behaviors = Behaviors.create(ApplicationSetup, partition: 'public-catalog', signal: BehaviorSignal());
    offset = (await page({'page': 1}) as num).toInt();
  }
}
