@Setup
class AnnotatedConfig {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return MemoryAdapters.build();
  }
}

@Settings({"prefix":"YON_APP_"})
class AnnotatedSettings {
    String? title;
    @Min(1)
    @Max(100)
    int size = 20;
}

class AnnotatedValues {
  @Setting("${yon.app.title:Untitled}")
  String title = '';
  @Setting({"path":"yon.app.size","type":"integer","default":5,"min":1,"max":100})
  int size = 0;
}

@Setup
@WhenProfile("development", "test")
@WhenSetting({"name":"YON_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
class AnnotatedConditionalConfig {
  static int calls = 0;

  static BehaviorAdapters configure(BehaviorExecution execution) {
    calls += 1;
    return MemoryAdapters.build();
  }
}

@Checked
class AnnotatedInput {
  @NotEmpty
  @Length({"min":2,"max":8})
  Object? name;
  @Required
  @Min(1)
  @Max(10)
  Object? amount;
  @Length({"min":1,"max":2})
  Object? optional;
}

class AnnotatedValidationService {
  int calls = 0;

  Object? accept(@Check(AnnotatedInput) Object? input) {
    calls += 1;
    return input;
  }
}

@Checked
class AnnotatedPageInput {
  @Required
  @Min(1)
  Object? size;
}

@CacheSettings({"name":"products","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
class AnnotatedValidatedCache {
  @Needs
  final BehaviorContext behaviors;
  int calls = 0;

  @Cached
  Future<Object?> accept(@Check(AnnotatedInput) Object? input) async {
    calls += 1;
    return input;
  }

  Object? page(@Check(AnnotatedPageInput) @PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) Object? input) {
    calls += 1;
    return input;
  }

  @WebCall({"client":"catalog","method":"GET","path":"/products"})
  Future<Object?> remote(@Check(AnnotatedInput) Object? input) async {
    calls += 1;
    return <String, Object?>{};
  }
}

final class MemoryAdapters {
  static int authorizations = 0;
  static int reads = 0;

  static BehaviorAdapters build() {
    return BehaviorAdapters(
      caches: {
        'products': BehaviorCacheAdapter(
          get: (key, execution) async {
            reads += 1;
            return <String, Object?>{'hit': false};
          },
          set: (key, value, policy, expiresAt, execution) async {},
          delete: (key, execution) async {},
        ),
      },
      http: {
        'catalog': BehaviorHttpAdapter(exchange: (request, execution) async => <String, Object?>{'transport': true}),
      },
      authorize: (call, execution) async {
        authorizations += 1;
        return true;
      },
    );
  }
}

void check(bool holds, String code) {
  if (!holds) throw StateError(code);
}

bool same(Object? left, Object? right) => BehaviorJson.encode(left) == BehaviorJson.encode(right);

Future<void> expectValidation(Object? Function() body, String path, String code) async {
  try {
    final result = body();
    if (result is Future) await result;
  } on BehaviorError catch (error) {
    check(error.code == 'validation' && (path.isEmpty || error.path == path), '$code: ${error.code} ${error.path}');
    return;
  }
  throw StateError(code);
}

void checkBinding() {
  final bound = Behaviors.properties(AnnotatedSettings, {'YON_APP_TITLE': 'Products'});
  check(same(bound, {'title': 'Products', 'size': 20}), 'GENERATED_PROPERTIES');
  final application = Behaviors.values(AnnotatedValues, {
    'yon': {
      'app': {'size': 12},
    },
  });
  check(same(application, {'title': 'Untitled', 'size': 12}), 'GENERATED_VALUES');
  try {
    Behaviors.values(AnnotatedValues, {'secrets': <String, Object?>{}});
    throw StateError('UNKNOWN_SECTION_ACCEPTED');
  } on BehaviorError catch (error) {
    check(error.code == 'configuration-properties', 'UNKNOWN_SECTION_CODE');
  }
  final manifest = Behaviors.values(AnnotatedValues, {
    'name': 'Catalog',
    'icons': [
      {'src': '/icon.svg'},
    ],
    'yon': {
      'app': {'size': 7},
    },
  });
  check(same(manifest, {'title': 'Untitled', 'size': 7}), 'MANIFEST_MEMBERS_IGNORED');
}

void checkConditionalConfiguration() {
  final signal = BehaviorSignal();
  try {
    Behaviors.create(AnnotatedConditionalConfig, partition: 'tenant-one', signal: signal).dispose();
    throw StateError('INACTIVE_ACCEPTED');
  } on BehaviorError catch (error) {
    check(error.code == 'inactive-configuration', 'INACTIVE_CODE');
  }
  check(AnnotatedConditionalConfig.calls == 0, 'INACTIVE_CALLED');
  final active = Behaviors.create(AnnotatedConditionalConfig,
      partition: 'tenant-one', signal: signal, profiles: ['test'], properties: {'YON_CACHE_ENABLED': 'true'});
  active.dispose();
  check(AnnotatedConditionalConfig.calls == 1, 'ACTIVE_NOT_CALLED');
}

BehaviorContext createShared(String partition) => Behaviors.create(AnnotatedConditionalConfig,
    partition: partition, signal: BehaviorSignal(), profiles: ['test'], properties: {'YON_CACHE_ENABLED': 'true'});

void checkSharedSetup() {
  final configuredBefore = AnnotatedConditionalConfig.calls;
  final writer = createShared('writer');
  final reader = createShared('reader');
  check(AnnotatedConditionalConfig.calls == configuredBefore + 1, 'SETUP_CONFIGURED_TWICE');
  writer.dispose();
  writer.dispose();
  final later = createShared('later');
  check(AnnotatedConditionalConfig.calls == configuredBefore + 1, 'SETUP_CONFIGURED_WHILE_OPEN');
  reader.dispose();
  later.dispose();
  createShared('fresh').dispose();
  check(AnnotatedConditionalConfig.calls == configuredBefore + 2, 'SETUP_NOT_CONFIGURED_AGAIN');
}

Future<void> checkServiceValidation(Map<String, Object?> valid) async {
  final service = AnnotatedValidationService();
  check(same(Behaviors.validate(AnnotatedInput, valid), valid) && same(service.accept(valid), valid), 'VALIDATION_MUTATED');
  final invalidInputs = [
    (<String, Object?>{'name': 'é'}, 'amount.notNull'),
    (<String, Object?>{'name': 'é', 'amount': 0}, 'amount.min'),
    (<String, Object?>{'name': 'é', 'amount': 11}, 'amount.max'),
    (<String, Object?>{'name': 'é', 'amount': '1'}, 'amount.min'),
    (<String, Object?>{'name': 'x', 'amount': 1}, 'name.size'),
    (<String, Object?>{'name': 'é', 'amount': 1, 'optional': <Object?>[]}, 'optional.size'),
    (<Object?>[], r'$.object'),
  ];
  for (final (input, path) in invalidInputs) {
    await expectValidation(() => service.accept(input), path, 'INVALID_ACCEPTED $path');
  }
  final blankNames = [
    '',
    '\t\n\r ',
    '\u{0085}\u{00a0}\u{1680}\u{2000}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}',
  ];
  for (final whitespace in blankNames) {
    await expectValidation(() => service.accept({'name': whitespace, 'amount': 1}), 'name.notBlank', 'BLANK_ACCEPTED');
  }
  check(service.calls == 1, 'INVALID_BODY_RAN');
}

Future<void> checkDirectValidation(Map<String, Object?> valid) async {
  for (final invalid in [double.nan, {'large': 'x' * 1048577}]) {
    await expectValidation(() => Behaviors.validate(AnnotatedInput, invalid), r'$.json', 'INVALID_JSON_ACCEPTED');
  }
  Behaviors.validate(AnnotatedInput, {'name': 'é', 'amount': 1, 'optional': null});
  Behaviors.validate(AnnotatedInput, {'name': 'é', 'amount': 1, 'optional': {'a': true}});
  Behaviors.validate(AnnotatedInput, {'name': '\u200b', 'amount': 1});
  await expectValidation(() => Behaviors.validate(AnnotatedConfig, valid), r'$.schema', 'UNKNOWN_SCHEMA_ACCEPTED');
}

Future<void> checkCachedValidation() async {
  final context = Behaviors.create(AnnotatedConfig, partition: 'tenant-validation', signal: BehaviorSignal());
  try {
    final cached = AnnotatedValidatedCache(context);
    final authorizations = MemoryAdapters.authorizations;
    final reads = MemoryAdapters.reads;
    await expectValidation(() => cached.accept(<String, Object?>{}), '', 'INVALID_CACHE_ACCEPTED');
    check(cached.calls == 0 && authorizations == MemoryAdapters.authorizations && reads == MemoryAdapters.reads,
        'INVALID_CACHE_BODY_ADAPTER_RAN');
    await expectValidation(() => cached.page({'size': 0}), 'size.min', 'INVALID_PAGE_ACCEPTED');
    await expectValidation(() => cached.remote(<String, Object?>{}), '', 'INVALID_HTTP_ACCEPTED');
    check(cached.calls == 0 && (cached.page({'size': 2, 'page': 3}) as Map)['offset'] == 6, 'VALIDATION_PAGE_HTTP_ORDER');
  } finally {
    context.dispose();
  }
}

Future<void> main() async {
  checkBinding();
  checkConditionalConfiguration();
  checkSharedSetup();
  final valid = <String, Object?>{'name': 'é', 'amount': 1, 'extra': true};
  await checkServiceValidation(valid);
  await checkDirectValidation(valid);
  await checkCachedValidation();
  print('Dart emitted configuration binding and validation executed');
}
