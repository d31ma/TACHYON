@Setup
class AnnotatedConfig {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return MemoryAdapters.build();
  }
}

@Entity({"adapter":"data"})
@Table("products")
class AnnotatedProduct {
  @Id
  int id = 0;
}

@CacheSettings({"name":"products","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
class AnnotatedCatalog {
  @Needs
  final BehaviorContext behaviors;
  int calls = 0;

  @Cached
  Future<Object?> list(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) Object? page) async {
    calls += 1;
    return {'calls': calls, 'offset': (page as Map)['offset']};
  }

  @WebCall({"client":"catalog","method":"GET","path":"/products"})
  Future<Object?> remote() async {
    return <String, Object?>{};
  }

  @WebCall({"client":"catalog","method":"GET","path":"/products","query":{"page":0}})
  Future<Object?> search(Object? page) async {}

  Object? page(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) Object? input) {
    return input;
  }

  @UpdateCache({"method":"list"})
  Future<Object?> update(Object? value, Object? page) async {
    return value;
  }

  @ClearCache({"method":"list"})
  Future<Object?> remove(Object? page) async {
    return null;
  }
}

@Store(AnnotatedProduct, {"fields":["id","name"],"defaultSize":20,"maxSize":100})
class AnnotatedQueries {
  @Needs
  final BehaviorContext behaviors;

  @Query({"operation":"find","where":[{"field":"name","operator":"eq","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
  Future<Object?> findNamed(Object? name, @PageDefaults Object? page) async {}

  Future<Object?> countByName(Object? name) async {}

  Future<Object?> existsByName(Object? name) async {}

  Future<Object?> findById(Object? id) async {}

  Future<Object?> findAll(Object? page) async {}

  Future<Object?> deleteById(Object? id) async {}

  Future<Object?> deleteByName(Object? name) async {}

  Future<Object?> save(Object? entity) => behaviors.repository(AnnotatedProduct).save(entity);
}

abstract class AnnotatedSummaries {
  @Convert({"fields":[{"source":"detail.id","target":"productId"},{"source":"name","target":"title"},{"source":"missing.deep","target":"absent"},{"target":"kind","constant":"product"}]})
  Object? toSummary(Object? product);
}

final class TransientFailure implements Exception {}

class AnnotatedLedger {
  @Needs
  final BehaviorContext behaviors;
  late BehaviorContext entries;

  @AllOrNothing({"adapter":"data"})
  Future<Object?> settle(Object? second) async {
    await entries.repository(AnnotatedProduct).save(<String, Object?>{'id': 'first'});
    await entries.repository(AnnotatedProduct).save(<String, Object?>{'id': second});
    return 'settled';
  }
}

class AnnotatedResilient {
  @Needs
  final BehaviorContext behaviors;
  int attempts = 0;

  @Retry({"maxRetries":3,"delayMs":0,"multiplier":1,"maxDelayMs":50,"jitterMs":0})
  Future<Object?> flaky() async {
    attempts += 1;
    if (attempts < 3) throw TransientFailure();
    return attempts;
  }

  @MaxAtOnce({"limit":2,"policy":"block"})
  Future<Object?> bounded(Object? value) async {
    return value;
  }
}

final class MemoryAdapters {
  static final Map<String, Map<String, Object?>> _entries = {};
  static final List<String> ledger = [];
  static int transactions = 0;

  static Future<Object?> readEntry(String key, BehaviorExecution execution) async {
    final entry = _entries[key];
    if (entry == null) {
      return <String, Object?>{'hit': false};
    }
    return <String, Object?>{'hit': true, 'value': entry['value'], 'expiresAt': entry['expiresAt']};
  }

  static Future<void> writeEntry(String key, Object? value, Object? policy, int expiresAt, BehaviorExecution execution) async {
    _entries[key] = {'value': value, 'expiresAt': expiresAt};
  }

  static Future<void> deleteEntry(String key, BehaviorExecution execution) async {
    _entries.remove(key);
  }

  static Future<Object?> executeEntity(Object? command, BehaviorExecution execution) async {
    final fields = command as Map;
    switch (fields['operation']) {
      case 'transaction':
        return transaction(fields);
      case 'query':
        return queryResult((fields['query'] as Map)['kind']);
      default:
        return persist(fields);
    }
  }

  static Object? transaction(Map fields) {
    final joined = fields['transaction'];
    ledger.add(joined == null ? '${fields['action']}' : '${fields['action']}:$joined');
    if (fields['action'] != 'begin') return <String, Object?>{};
    transactions += 1;
    return <String, Object?>{'transaction': 't$transactions'};
  }

  static Object? persist(Map fields) {
    ledger.add('${fields['operation']}@${fields['transaction'] ?? 'none'}');
    final value = fields['value'];
    if (value is Map && value['id'] == 'broken') throw StateError('the second write failed');
    return <String, Object?>{...fields.cast<String, Object?>()};
  }

  static Object? queryResult(Object? kind) {
    switch (kind) {
      case 'find':
        return [<String, Object?>{'id': 1, 'name': 'tools'}];
      case 'count' || 'delete':
        return 1;
      default:
        return true;
    }
  }

  static BehaviorAdapters build() {
    return BehaviorAdapters(
      caches: {
        'products': BehaviorCacheAdapter(get: readEntry, set: writeEntry, delete: deleteEntry),
      },
      http: {
        'catalog': BehaviorHttpAdapter(
          exchange: (request, execution) async => <String, Object?>{'transport': true, 'request': request},
        ),
      },
      entities: {
        'data': BehaviorEntityAdapter(execute: executeEntity),
      },
      authorize: (call, execution) async => true,
      authorizeEntity: (command, execution) async => true,
    );
  }
}

void check(bool holds, String code) {
  if (!holds) throw StateError(code);
}

bool same(Object? left, Object? right) => BehaviorJson.encode(left) == BehaviorJson.encode(right);

Future<void> checkCatalog(BehaviorContext context, Map<String, Object?> input) async {
  final catalog = AnnotatedCatalog(context);
  final first = await catalog.list(input);
  final second = await catalog.list(input);
  check(catalog.calls == 1 && same(first, second) && (first as Map)['offset'] == 6, 'GENERATED_CACHE_PAGE');
  await context.evict('AnnotatedCatalog', 'list', [input]);
  await catalog.list(input);
  check(catalog.calls == 2 && (catalog.page(input) as Map)['offset'] == 6, 'GENERATED_EVICTION');
  check((await catalog.remote() as Map)['transport'] == true, 'GENERATED_HTTP');
  final searched = await catalog.search(2) as Map;
  check(((searched['request'] as Map)['query'] as Map)['page'] == 2, 'GENERATED_DECLARED_EXCHANGE');
  final saved = await context.repository(AnnotatedProduct).save(<String, Object?>{'id': 4}) as Map;
  check(saved['table'] == 'products', 'GENERATED_ENTITY');
  final replacement = <String, Object?>{'updated': true};
  await catalog.update(replacement, input);
  check(same(await catalog.list(input), replacement) && catalog.calls == 2, 'GENERATED_PUT');
  await catalog.remove(input);
  await catalog.list(input);
  check(catalog.calls == 3, 'GENERATED_EVICT');
}

Future<void> checkResilience(BehaviorContext context) async {
  final resilient = AnnotatedResilient(context);
  check(await resilient.flaky() == 3 && resilient.attempts == 3, 'GENERATED_RETRY');
  check(await resilient.bounded('ok') == 'ok', 'GENERATED_LIMIT');
}

Future<void> checkQueries(BehaviorContext context, Map<String, Object?> input) async {
  final queries = AnnotatedQueries(context);
  check((await queries.findNamed('tools', input) as List).length == 1, 'GENERATED_QUERY');
  check(await queries.countByName('tools') == 1 && await queries.existsByName('tools') == true, 'GENERATED_DERIVED_QUERY');
  check((await queries.findById(1) as Map)['name'] == 'tools', 'REPOSITORY_FIND_BY_ID');
  check((await queries.findAll(input) as List).length == 1, 'REPOSITORY_FIND_ALL');
  check((await queries.save(<String, Object?>{'id': 1}) as Map)['operation'] == 'save', 'REPOSITORY_SAVE');
  check(await queries.deleteById(1) == 1 && await queries.deleteByName('tools') == 1, 'REPOSITORY_DELETE');
}

void checkSummaries(BehaviorContext context) {
  final summaries = AnnotatedSummaries();
  final summary = summaries.toSummary(<String, Object?>{
    'detail': {'id': 7},
    'name': 'Anvil',
    'extra': 'dropped',
  });
  check(same(summary, {'productId': 7, 'title': 'Anvil', 'absent': null, 'kind': 'product'}), 'GENERATED_MAPPING');
  try {
    summaries.toSummary(<Object?>[]);
    throw StateError('GENERATED_MAPPING_OBJECT');
  } on BehaviorError catch (error) {
    check(error.code == 'JSON_OBJECT', 'GENERATED_MAPPING_OBJECT');
  }
}

Future<void> checkJoinedTransaction(BehaviorSignal signal) async {
  final service = Behaviors.create(AnnotatedConfig, partition: 'ledger', signal: signal);
  final store = Behaviors.create(AnnotatedConfig, partition: 'entries', signal: signal);
  try {
    final ledger = AnnotatedLedger(service)..entries = store;
    MemoryAdapters.ledger.clear();
    check(await ledger.settle('second') == 'settled', 'JOINED_RESULT');
    check(MemoryAdapters.ledger.join(',') == 'begin,save@t1,save@t1,commit:t1', 'JOINED_TRANSACTION');
    MemoryAdapters.ledger.clear();
    try {
      await ledger.settle('broken');
      check(false, 'JOINED_FAILURE');
    } on BehaviorError catch (error) {
      check(error.code == 'ADAPTER_FAILED', 'JOINED_FAILURE_CODE');
    }
    check(MemoryAdapters.ledger.join(',') == 'begin,save@t2,save@t2,rollback:t2', 'JOINED_ROLLBACK');
  } finally {
    service.dispose();
    store.dispose();
  }
}

Future<void> main() async {
  final signal = BehaviorSignal();
  final context = Behaviors.create(AnnotatedConfig, partition: 'tenant-one', signal: signal);
  try {
    final input = <String, Object?>{'page': 2, 'size': 3};
    await checkCatalog(context, input);
    await checkResilience(context);
    await checkQueries(context, input);
    checkSummaries(context);
    await checkJoinedTransaction(signal);
  } finally {
    context.dispose();
  }
  print('Dart emitted annotations: behavior decorators, cache mutations, Query, derived queries and Mapping executed');
}
