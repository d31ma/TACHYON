import 'dart:async' as behavior_async;
import 'dart:convert' as behavior_convert;
import 'dart:math' as behavior_math;
//__BEHAVIOR_BODY__

final class BehaviorError implements Exception {
  BehaviorError(this.code, [String? path])
      : path = path != null && behavior_convert.utf8.encode(path).length <= 128 ? path : null,
        message = 'Behavior operation failed: $code';
  BehaviorError.gateway(this.code)
      : path = null,
        message = 'Gateway call failed.';
  BehaviorError.storage(this.message)
      : code = 'STORAGE',
        path = null;
  final String code;
  final String? path;
  final String message;
  @override
  String toString() => message;
}

abstract final class BehaviorJson {
  static const int limit = 1048576;
  static const num _safe = 9007199254740991;

  static Object? from(Object? value) {
    encode(value);
    return value;
  }

  static num checkedNumber(num value) {
    if (value is double && !value.isFinite) throw BehaviorError('JSON_NUMBER');
    if (value.abs() > _safe) throw BehaviorError('JSON_NUMBER');
    return value;
  }

  static int integer(Object? value) {
    if (value is int && value.abs() <= _safe) return value;
    if (value is double && value.isFinite && value == value.roundToDouble() && value.abs() <= _safe) return value.toInt();
    throw BehaviorError('JSON_INTEGER');
  }

  static String text(Object? value) => value is String ? value : throw BehaviorError('JSON_STRING');
  static bool boolean(Object? value) => value is bool ? value : throw BehaviorError('JSON_BOOLEAN');
  static List<Object?> items(Object? value) => value is List ? value.cast<Object?>() : throw BehaviorError('JSON_ARRAY');
  static Map<String, Object?> members(Object? value) =>
      value is Map ? value.cast<String, Object?>() : throw BehaviorError('JSON_OBJECT');
  static Object? get(Object? value, String key) => value is Map ? value[key] : null;
  static bool has(Object? value, String key) => value is Map && value.containsKey(key);
  static void keys(Object? value, List<String> allowed) {
    for (final key in members(value).keys) {
      if (!allowed.contains(key)) throw BehaviorError('UNKNOWN_FIELD');
    }
  }

  static Object? parse(String source) {
    if (behavior_convert.utf8.encode(source).length > limit) throw BehaviorError('JSON_LIMIT');
    final Object? value;
    try {
      value = behavior_convert.jsonDecode(source);
    } on FormatException {
      throw BehaviorError('JSON_INVALID');
    }
    encode(value);
    return value;
  }

  static String encode(Object? value, [int maximum = limit]) {
    final buffer = StringBuffer();
    _write(buffer, value, 0, maximum);
    return buffer.toString();
  }

  static void _write(StringBuffer out, Object? value, int depth, int maximum) {
    if (depth > 32) throw BehaviorError('JSON_DEPTH');
    if (value == null) {
      out.write('null');
    } else if (value is bool) {
      out.write(value ? 'true' : 'false');
    } else if (value is num) {
      checkedNumber(value);
      out.write(value is double && value == value.roundToDouble() ? value.toInt() : value);
    } else if (value is String) {
      out.write(behavior_convert.jsonEncode(value));
    } else if (value is List) {
      out.write('[');
      for (var index = 0; index < value.length; index++) {
        if (index > 0) out.write(',');
        _write(out, value[index], depth + 1, maximum);
      }
      out.write(']');
    } else if (value is Map) {
      final keys = value.keys.map((key) => key is String ? key : throw BehaviorError('JSON_TYPE')).toList()..sort();
      out.write('{');
      for (var index = 0; index < keys.length; index++) {
        if (index > 0) out.write(',');
        out
          ..write(behavior_convert.jsonEncode(keys[index]))
          ..write(':');
        _write(out, value[keys[index]], depth + 1, maximum);
      }
      out.write('}');
    } else {
      throw BehaviorError('JSON_TYPE');
    }
    if (out.length > maximum) throw BehaviorError('JSON_LIMIT');
  }
}

final class BehaviorSignal {
  BehaviorSignal() : _parent = null;
  BehaviorSignal._child(BehaviorSignal parent) : _parent = parent;
  final BehaviorSignal? _parent;
  bool _cancelled = false;
  bool get aborted => _cancelled || (_parent?.aborted ?? false);
  void abort() => _cancelled = true;
}

final class BehaviorExecution {
  BehaviorExecution(this.environment, this.partition, this.signal, this.owner, this.deadline);
  final String environment;
  final String partition;
  final BehaviorSignal signal;
  final BehaviorSignal owner;
  final int deadline;
  void check() {
    if (signal.aborted || owner.aborted) throw BehaviorError('CANCELLED');
    if (Behaviors.now >= deadline) throw BehaviorError('DEADLINE');
  }
}

final class BehaviorCacheAdapter {
  BehaviorCacheAdapter({required this.get, required this.set, required this.delete});
  final Future<Object?> Function(String key, BehaviorExecution execution) get;
  final Future<void> Function(String key, Object? value, Object? policy, int expiresAt, BehaviorExecution execution) set;
  final Future<void> Function(String key, BehaviorExecution execution) delete;
}

final class BehaviorHttpAdapter {
  BehaviorHttpAdapter({required this.exchange});
  final Future<Object?> Function(Object? request, BehaviorExecution execution) exchange;
}

final class BehaviorEntityAdapter {
  BehaviorEntityAdapter({required this.execute});
  final Future<Object?> Function(Object? command, BehaviorExecution execution) execute;
}

final class BehaviorStorage {
  static String Function(String request)? host;

  static bool get available => Behaviors.environment == 'tac' && host != null;

  static Object? ask(String operation, Map<String, Object?> request) {
    final answer = BehaviorJson.parse(host!(BehaviorJson.encode({...request, 'op': operation})));
    final error = BehaviorJson.get(answer, 'error');
    if (error is String) throw BehaviorError.storage(error);
    return BehaviorJson.get(answer, 'value');
  }

  static BehaviorCacheAdapter? cache(String name) {
    if (!available) return null;
    return BehaviorCacheAdapter(
      get: (key, execution) async => ask('cache.get', {'cache': name, 'key': key}),
      set: (key, value, policy, expiresAt, execution) async {
        ask('cache.set', {
          'cache': name,
          'key': key,
          'value': value,
          'expiresAt': expiresAt,
          'maxEntries': BehaviorJson.get(policy, 'maxEntries'),
        });
      },
      delete: (key, execution) async {
        ask('cache.delete', {'cache': name, 'key': key});
      },
    );
  }

  static BehaviorEntityAdapter? entities(String name) {
    if (!available || name != 'indexeddb') return null;
    return BehaviorEntityAdapter(execute: (command, execution) async => ask('entity', {'command': command}));
  }
}

final class BehaviorAdapters {
  BehaviorAdapters({
    this.caches = const {},
    this.http = const {},
    this.entities = const {},
    this.authorize,
    this.authorizeEntity,
  });
  final Map<String, BehaviorCacheAdapter> caches;
  final Map<String, BehaviorHttpAdapter> http;
  final Map<String, BehaviorEntityAdapter> entities;
  final Future<bool> Function(Object? call, BehaviorExecution execution)? authorize;
  final Future<bool> Function(Object? command, BehaviorExecution execution)? authorizeEntity;
}

abstract final class BehaviorPage {
  static Object? normalize(Object? input, Object? policy) {
    final page = input ?? const <String, Object?>{};
    BehaviorJson.keys(page, const ['page', 'size', 'offset', 'sort']);
    final number = BehaviorJson.has(page, 'page') ? BehaviorJson.integer(BehaviorJson.get(page, 'page')) : 0;
    final size = BehaviorJson.has(page, 'size')
        ? BehaviorJson.integer(BehaviorJson.get(page, 'size'))
        : BehaviorJson.integer(BehaviorJson.get(policy, 'defaultSize'));
    final maximum = BehaviorJson.integer(BehaviorJson.get(policy, 'maxSize'));
    if (number < 0 || size < 1 || size > maximum || maximum > 1000 || number > 1000000 ~/ size) {
      throw BehaviorError('PAGE_INVALID');
    }
    final allowed = BehaviorJson.items(BehaviorJson.get(policy, 'sort')).map(BehaviorJson.text).toSet();
    final seen = <String>{};
    final sort = <Object?>[];
    for (final item in BehaviorJson.has(page, 'sort') ? BehaviorJson.items(BehaviorJson.get(page, 'sort')) : const []) {
      final parts = BehaviorJson.text(item).split(':');
      final field = parts[0];
      final direction = parts.length == 1 ? 'asc' : parts[1];
      if (parts.length > 2 || !allowed.contains(field) || !seen.add(field) || !const ['asc', 'desc'].contains(direction)) {
        throw BehaviorError('PAGE_SORT');
      }
      sort.add('$field:$direction');
    }
    return <String, Object?>{'page': number, 'size': size, 'offset': number * size, 'sort': sort};
  }
}

abstract final class Behaviors {
  static int get now => DateTime.now().millisecondsSinceEpoch;
  static const String module = __TAC_BEHAVIOR_MODULE__;
  static const String environment = __TAC_BEHAVIOR_ENVIRONMENT__;
  static final Object? _registry = BehaviorJson.parse(__TAC_BEHAVIOR_REGISTRY__);
  static final RegExp _name = RegExp(r'^[A-Za-z_$][A-Za-z0-9_$]{0,63}$');
  static final RegExp _upper = RegExp(r'^[A-Z][A-Z0-9_]*$');

  static const List<String> _gatewayFailures = ['invalid', 'timeout', 'capacity', 'adapter'];

  static Future<Object?> gatewayCall(
      String key, List<Object?> arguments, Future<String> Function(String key, String arguments) transport) async {
    final String encoded;
    try {
      encoded = BehaviorJson.encode(arguments);
    } on BehaviorError {
      throw BehaviorError.gateway('invalid');
    }
    return gatewayResult(await transport(key, encoded));
  }

  static Object? gatewayResult(String answer) {
    final Object? decoded;
    try {
      decoded = behavior_convert.jsonDecode(answer);
    } on FormatException {
      throw BehaviorError.gateway('adapter');
    }
    if (decoded is! Map) throw BehaviorError.gateway('adapter');
    if (decoded.containsKey('result')) return decoded['result'];
    final kind = decoded['error'];
    if (kind is String && _gatewayFailures.contains(kind)) throw BehaviorError.gateway(kind);
    throw BehaviorError.gateway('adapter');
  }

  static Object? registration(String type) {
    final value = BehaviorJson.get(_registry, type);
    if (value == null) throw BehaviorError('UNREGISTERED');
    return value;
  }

  static Object? entity(Type entity) {
__TAC_BEHAVIOR_ENTITIES__
    throw BehaviorError('ENTITY_UNKNOWN');
  }

  static Object? _typePolicies(Type type) {
__TAC_BEHAVIOR_TYPES__
    return const <String, Object?>{};
  }

  static BehaviorAdapters _configure(Type configuration, BehaviorExecution execution) {
__TAC_BEHAVIOR_CONFIGURATIONS__
    throw BehaviorError('CONFIGURATION_UNKNOWN');
  }

  static Object? validate(Type type, Object? value) {
    try {
      return validateSchema(BehaviorJson.get(_typePolicies(type), 'validated'), value);
    } on BehaviorError catch (error) {
      if (error.code == 'validation') rethrow;
      throw BehaviorError('validation', r'$.schema');
    }
  }

  static bool _blank(String value) => value.runes.every((code) =>
      (code >= 9 && code <= 13) ||
      const [32, 133, 160, 5760, 8232, 8233, 8239, 8287, 12288].contains(code) ||
      (code >= 8192 && code <= 8202));

  static Object? validateSchema(Object? schema, Object? value) {
    try {
      BehaviorJson.encode(value);
    } on BehaviorError {
      throw BehaviorError('validation', r'$.json');
    }
    if (value is! Map) throw BehaviorError('validation', r'$.object');
    try {
      BehaviorJson.keys(schema, const ['fields']);
      for (final entry in BehaviorJson.members(BehaviorJson.get(schema, 'fields')).entries) {
        final field = entry.key;
        final rules = entry.value;
        if (!_name.hasMatch(field)) throw BehaviorError('validation', r'$.schema');
        BehaviorJson.keys(rules, const ['notNull', 'notBlank', 'size', 'min', 'max']);
        final item = value[field];
        if (BehaviorJson.has(rules, 'notNull') && BehaviorJson.boolean(BehaviorJson.get(rules, 'notNull')) && item == null) {
          throw BehaviorError('validation', '$field.notNull');
        }
        if (BehaviorJson.has(rules, 'notBlank') && BehaviorJson.boolean(BehaviorJson.get(rules, 'notBlank'))) {
          if (item is! String || _blank(item)) throw BehaviorError('validation', '$field.notBlank');
        }
        if (item == null) continue;
        if (BehaviorJson.has(rules, 'size')) {
          final size = BehaviorJson.get(rules, 'size');
          BehaviorJson.keys(size, const ['min', 'max']);
          final count = item is String
              ? behavior_convert.utf8.encode(item).length
              : item is List
                  ? item.length
                  : item is Map
                      ? item.length
                      : -1;
          final minimum = BehaviorJson.has(size, 'min') ? BehaviorJson.integer(BehaviorJson.get(size, 'min')) : 0;
          final maximum = BehaviorJson.has(size, 'max') ? BehaviorJson.integer(BehaviorJson.get(size, 'max')) : 1048576;
          if (minimum < 0 || maximum < minimum) throw BehaviorError('validation', r'$.schema');
          if (count < minimum || count > maximum) throw BehaviorError('validation', '$field.size');
        }
        for (final rule in const ['min', 'max']) {
          if (!BehaviorJson.has(rules, rule)) continue;
          if (item is! num) throw BehaviorError('validation', '$field.$rule');
          final bound = BehaviorJson.get(rules, rule);
          if (bound is! num) throw BehaviorError('validation', r'$.schema');
          BehaviorJson.checkedNumber(bound);
          if ((rule == 'min' && item < bound) || (rule == 'max' && item > bound)) {
            throw BehaviorError('validation', '$field.$rule');
          }
        }
      }
      return value;
    } on BehaviorError catch (error) {
      if (error.code == 'validation') rethrow;
      throw BehaviorError('validation', r'$.schema');
    }
  }

  static String get _frameworkPrefix => environment == 'tac' ? 'TAC_' : 'YON_';

  static BehaviorContext create(
    Type configuration, {
    required String partition,
    required BehaviorSignal signal,
    int timeoutMs = 10000,
    List<String> profiles = const [],
    Map<String, Object?> properties = const {},
  }) {
    if (partition.isEmpty || behavior_convert.utf8.encode(partition).length > 256 || timeoutMs < 1 || timeoutMs > 60000) {
      throw BehaviorError('CONTEXT_INVALID');
    }
    final execution = BehaviorExecution(environment, partition, signal, signal, now + timeoutMs)..check();
    final selected = _snapshotProfiles(profiles);
    final values = _propertySource(properties, 'configuration');
    _activeConfiguration(_typePolicies(configuration), selected, values);
    final BehaviorAdapters adapters;
    try {
      adapters = _configure(configuration, execution);
    } on BehaviorError {
      rethrow;
    } catch (_) {
      throw BehaviorError('CONFIGURATION_FAILED');
    }
    execution.check();
    return BehaviorContext._(adapters, partition, signal, timeoutMs);
  }

  static Map<String, String> _propertySource(Map<String, Object?> input, String code) {
    if (input.containsKey('tac') || input.containsKey('yon')) {
      final Map<String, String> flat;
      try {
        flat = _applicationInputs(input);
      } on BehaviorError {
        throw BehaviorError(code);
      }
      return _snapshotProperties({
        for (final entry in flat.entries)
          if (entry.key.startsWith('$environment.')) entry.key.replaceAll('.', '_').toUpperCase(): entry.value,
      }, code);
    }
    final values = <String, String>{};
    for (final entry in input.entries) {
      final value = entry.value;
      if (value is! String) throw BehaviorError(code);
      values[entry.key] = value;
    }
    return _snapshotProperties(values, code);
  }

  static List<String> _snapshotProfiles(List<String> profiles) {
    if (profiles.length > 32) throw BehaviorError('configuration');
    final seen = <String>{};
    for (final profile in profiles) {
      if (!_name.hasMatch(profile) || !seen.add(profile)) throw BehaviorError('configuration');
    }
    return List.unmodifiable(profiles);
  }

  static Map<String, String> _snapshotProperties(Map<String, String> values, String code) {
    if (values.length > 256) throw BehaviorError(code);
    for (final entry in values.entries) {
      if (behavior_convert.utf8.encode(entry.key).length > 256 ||
          !entry.key.startsWith(_frameworkPrefix) ||
          !_upper.hasMatch(entry.key) ||
          behavior_convert.utf8.encode(entry.value).length > 4096) {
        throw BehaviorError(code);
      }
    }
    try {
      BehaviorJson.encode(values, 65536);
    } on BehaviorError {
      throw BehaviorError(code);
    }
    return Map.unmodifiable(values);
  }

  static void _activeConfiguration(Object? policies, List<String> profiles, Map<String, String> values) {
    if (BehaviorJson.has(policies, 'profiles')) {
      final allowed = BehaviorJson.items(BehaviorJson.get(policies, 'profiles'));
      if (allowed.isEmpty || allowed.length > 32) throw BehaviorError('configuration');
      var active = false;
      for (final profile in allowed) {
        final name = BehaviorJson.text(profile);
        if (!_name.hasMatch(name)) throw BehaviorError('configuration');
        if (profiles.contains(name)) active = true;
      }
      if (!active) throw BehaviorError('inactive-configuration');
    }
    if (BehaviorJson.has(policies, 'conditionalOnProperty')) {
      final condition = BehaviorJson.get(policies, 'conditionalOnProperty');
      BehaviorJson.keys(condition, const ['name', 'havingValue', 'matchIfMissing']);
      final name = BehaviorJson.text(BehaviorJson.get(condition, 'name'));
      final expected = BehaviorJson.text(BehaviorJson.get(condition, 'havingValue'));
      if (behavior_convert.utf8.encode(name).length > 256 ||
          !name.startsWith(_frameworkPrefix) ||
          !_upper.hasMatch(name) ||
          behavior_convert.utf8.encode(expected).length > 4096) {
        throw BehaviorError('configuration');
      }
      if (values.keys.any((key) => key.startsWith('${name}_'))) throw BehaviorError('inactive-configuration');
      final supplied = values[name];
      final active = supplied != null
          ? supplied == expected
          : BehaviorJson.boolean(BehaviorJson.get(condition, 'matchIfMissing'));
      if (!active) throw BehaviorError('inactive-configuration');
    }
  }

  static Object? properties(Type type, Map<String, Object?> values) {
    try {
      final supplied = _propertySource(values, 'configuration-properties');
      final schema = BehaviorJson.get(_typePolicies(type), 'configurationProperties');
      BehaviorJson.keys(schema, const ['prefix', 'fields']);
      final prefix = BehaviorJson.text(BehaviorJson.get(schema, 'prefix'));
      if (behavior_convert.utf8.encode(prefix).length > 128 ||
          !prefix.startsWith(_frameworkPrefix) ||
          !prefix.endsWith('_') ||
          !_upper.hasMatch(prefix)) {
        throw BehaviorError('configuration-properties');
      }
      final known = <String>{};
      final result = <String, Object?>{};
      for (final entry in BehaviorJson.members(BehaviorJson.get(schema, 'fields')).entries) {
        final descriptor = entry.value;
        if (!_name.hasMatch(entry.key)) throw BehaviorError('configuration-properties');
        BehaviorJson.keys(descriptor, const ['name', 'type', 'default', 'min', 'max']);
        final suffix = BehaviorJson.text(BehaviorJson.get(descriptor, 'name'));
        final kind = BehaviorJson.text(BehaviorJson.get(descriptor, 'type'));
        final key = '$prefix$suffix';
        if (!_upper.hasMatch(suffix) || behavior_convert.utf8.encode(key).length > 256 || !known.add(key)) {
          throw BehaviorError('configuration-properties');
        }
        final raw = supplied[key];
        final Object? value;
        if (raw != null) {
          value = _parseProperty(raw, kind);
        } else if (BehaviorJson.has(descriptor, 'default')) {
          value = BehaviorJson.get(descriptor, 'default');
        } else {
          throw BehaviorError('configuration-properties');
        }
        _validateProperty(value, descriptor);
        result[entry.key] = value;
      }
      for (final key in supplied.keys) {
        if (key.startsWith(prefix) && !known.contains(key)) throw BehaviorError('configuration-properties');
      }
      BehaviorJson.encode(result, 65536);
      return result;
    } catch (_) {
      throw BehaviorError('configuration-properties');
    }
  }

  static Map<String, String> _applicationInputs(Object? application) {
    final flat = <String, String>{};
    BehaviorJson.encode(application, 65536);
    final members = BehaviorJson.members(application);
    if (!members.containsKey('tac') && !members.containsKey('yon')) throw BehaviorError('configuration-properties');
    for (final entry in members.entries) {
      if (entry.key != 'tac' && entry.key != 'yon') continue;
      BehaviorJson.members(entry.value);
      _flattenApplication(entry.value, entry.key, 0, flat);
    }
    return flat;
  }

  static final RegExp _section = RegExp(r'^[a-z][a-z0-9]*$');

  static void _flattenApplication(Object? value, String path, int depth, Map<String, String> flat) {
    if (depth > 16 || behavior_convert.utf8.encode(path).length > 256 || flat.length >= 256) {
      throw BehaviorError('configuration-properties');
    }
    if (path == 'yon.server.port') BehaviorJson.integer(value);
    if (value is Map) {
      if (depth > 0 && value.isEmpty) flat['$path.'] = '';
      for (final entry in BehaviorJson.members(value).entries) {
        if (!_section.hasMatch(entry.key)) throw BehaviorError('configuration-properties');
        _flattenApplication(entry.value, '$path.${entry.key}', depth + 1, flat);
      }
      return;
    }
    if (value == null || value is List) {
      flat['$path.'] = '';
      return;
    }
    final String text;
    if (value is String) {
      text = value;
    } else if (value is bool) {
      text = value ? 'true' : 'false';
    } else if (value is num) {
      BehaviorJson.checkedNumber(value);
      text = value == value.roundToDouble() ? '${value.toInt()}' : '$value';
    } else {
      throw BehaviorError('configuration-properties');
    }
    if (behavior_convert.utf8.encode(text).length > 4096 || flat.length >= 256) {
      throw BehaviorError('configuration-properties');
    }
    flat[path] = text;
  }

  static Object? values(Type type, Object? application) {
    try {
      final flat = _applicationInputs(application);
      final schema = BehaviorJson.members(BehaviorJson.get(_typePolicies(type), 'values'));
      final result = <String, Object?>{};
      for (final entry in schema.entries) {
        final descriptor = entry.value;
        if (!RegExp(r'^[A-Za-z_$][A-Za-z0-9_$]{0,127}$').hasMatch(entry.key)) {
          throw BehaviorError('configuration-properties');
        }
        BehaviorJson.keys(descriptor, const ['path', 'type', 'default', 'min', 'max']);
        final path = BehaviorJson.text(BehaviorJson.get(descriptor, 'path'));
        final kind = BehaviorJson.text(BehaviorJson.get(descriptor, 'type'));
        if (flat.keys.any((key) => key.startsWith('$path.'))) throw BehaviorError('configuration-properties');
        final raw = flat[path];
        final Object? value;
        if (raw != null) {
          value = _parseProperty(raw, kind);
        } else if (BehaviorJson.has(descriptor, 'default')) {
          value = BehaviorJson.get(descriptor, 'default');
        } else {
          throw BehaviorError('configuration-properties');
        }
        _validateProperty(value, descriptor);
        result[entry.key] = value;
      }
      BehaviorJson.encode(result, 65536);
      return result;
    } catch (_) {
      throw BehaviorError('configuration-properties');
    }
  }

  static Object? _parseProperty(String raw, String type) {
    if (type == 'string') return raw;
    if (type == 'boolean') {
      if (raw != 'true' && raw != 'false') throw BehaviorError('configuration-properties');
      return raw == 'true';
    }
    final grammar = type == 'integer'
        ? RegExp(r'^-?(0|[1-9][0-9]*)$')
        : RegExp(r'^-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?$');
    if ((type != 'integer' && type != 'number') || !grammar.hasMatch(raw)) {
      throw BehaviorError('configuration-properties');
    }
    return BehaviorJson.parse(raw);
  }

  static void _validateProperty(Object? value, Object? descriptor) {
    final type = BehaviorJson.text(BehaviorJson.get(descriptor, 'type'));
    final numeric = type == 'integer' || type == 'number';
    if (!numeric && (BehaviorJson.has(descriptor, 'min') || BehaviorJson.has(descriptor, 'max'))) {
      throw BehaviorError('configuration-properties');
    }
    if (type == 'string') {
      if (behavior_convert.utf8.encode(BehaviorJson.text(value)).length > 4096) throw BehaviorError('configuration-properties');
      return;
    }
    if (type == 'boolean') {
      BehaviorJson.boolean(value);
      return;
    }
    if (!numeric || value is! num) throw BehaviorError('configuration-properties');
    BehaviorJson.checkedNumber(value);
    if (type == 'integer') BehaviorJson.integer(value);
    num minimum = -9007199254740991;
    num maximum = 9007199254740991;
    for (final bound in const ['min', 'max']) {
      if (!BehaviorJson.has(descriptor, bound)) continue;
      final limit = BehaviorJson.get(descriptor, bound);
      if (limit is! num) throw BehaviorError('configuration-properties');
      BehaviorJson.checkedNumber(limit);
      if (bound == 'min') {
        minimum = limit;
      } else {
        maximum = limit;
      }
    }
    if (minimum > maximum || value < minimum || value > maximum) throw BehaviorError('configuration-properties');
  }
}

final class BehaviorOpenTransactions {
  static const Symbol _key = #tachyonBehaviorTransactions;

  static Map<String, String> get _open =>
      (behavior_async.Zone.current[_key] as Map<String, String>?) ?? const <String, String>{};

  static String? of(String adapter) => _open[adapter];

  static Future<Object?> within(String adapter, String identifier, Future<Object?> Function() body) =>
      behavior_async.runZoned(body, zoneValues: {_key: <String, String>{..._open, adapter: identifier}});
}

class BehaviorContext {
  BehaviorContext._(this._adapters, this._partition, this._owner, this._timeoutMs) : _signal = BehaviorSignal._child(_owner);
  final BehaviorAdapters _adapters;
  final String _partition;
  final BehaviorSignal _owner;
  final BehaviorSignal _signal;
  final int _timeoutMs;
  int _active = 0;
  final Map<String, int> _slots = {};

  void dispose() => _signal.abort();

  BehaviorExecution _execution() =>
      BehaviorExecution(Behaviors.environment, _partition, _signal, _owner, Behaviors.now + _timeoutMs);

  void _acquire() {
    if (_active >= 64) throw BehaviorError('OPERATION_LIMIT');
    _active += 1;
  }

  T paginate<T>(T Function() body) {
    _acquire();
    try {
      final execution = _execution()..check();
      final result = body();
      execution.check();
      return result;
    } finally {
      _active -= 1;
    }
  }

  Future<Object?> _operation(Future<Object?> Function(BehaviorExecution execution) body) async {
    _acquire();
    try {
      final execution = _execution()..check();
      final result = await body(execution);
      execution.check();
      BehaviorJson.encode(result);
      return result;
    } finally {
      _active -= 1;
    }
  }

  Future<T> _adapter<T>(BehaviorExecution execution, Future<T> Function() body) async {
    execution.check();
    final T result;
    try {
      result = await body();
    } on BehaviorError {
      rethrow;
    } catch (_) {
      execution.check();
      throw BehaviorError('ADAPTER_FAILED');
    }
    execution.check();
    return result;
  }

  Object? _method(String type, String method) {
    final metadata = BehaviorJson.get(BehaviorJson.get(Behaviors.registration(type), 'methods'), method);
    if (metadata == null) throw BehaviorError('METHOD_UNKNOWN');
    return metadata;
  }

  Future<String> _authorize(String operation, String type, String method, Object? policy, Object? metadata,
      List<Object?> arguments, BehaviorExecution execution) async {
    final selected = BehaviorJson.items(BehaviorJson.get(BehaviorJson.get(metadata, 'cacheable'), 'keyArgs')).map((item) {
      final index = BehaviorJson.integer(item);
      if (index < 0 || index >= arguments.length) throw BehaviorError('CACHE_KEY');
      return arguments[index];
    }).toList();
    final key = BehaviorJson.encode(
        [_partition, BehaviorJson.get(policy, 'name'), Behaviors.module, type, method, selected], 4096);
    final call = {
      'operation': operation,
      'cache': BehaviorJson.get(policy, 'name'),
      'type': type,
      'method': method,
      'keyArguments': selected,
    };
    final authorize = _adapters.authorize;
    final cacheName = BehaviorJson.text(BehaviorJson.get(policy, 'name'));
    if (authorize == null && !_adapters.caches.containsKey(cacheName) && BehaviorStorage.cache(cacheName) != null) return key;
    if (authorize == null) throw BehaviorError('FORBIDDEN');
    if (!await _adapter(execution, () => authorize(call, execution))) throw BehaviorError('FORBIDDEN');
    return key;
  }

  BehaviorCacheAdapter _cache(Object? policy) {
    final name = BehaviorJson.text(BehaviorJson.get(policy, 'name'));
    return _adapters.caches[name] ?? BehaviorStorage.cache(name) ?? (throw BehaviorError('CACHE_MISSING'));
  }

  BehaviorEntityAdapter _entities(String name) =>
      _adapters.entities[name] ?? BehaviorStorage.entities(name) ?? (throw BehaviorError('ENTITY_MISSING'));

  Future<Object?> invoke(String type, String method, List<Object?> arguments, Future<Object?> Function() body) =>
      _operation((execution) async {
        final written = _method(type, method);
        final generated = BehaviorJson.has(BehaviorJson.get(written, 'httpExchange'), 'generated');
        final metadata = generated ? _withFilledPath(written, arguments) : written;
        final work = generated ? _declaredBody(written, arguments) : body;
        final cached = ['cacheable', 'cacheEvict', 'cachePut'].any((key) => BehaviorJson.has(metadata, key));
        if (generated && !cached) return _body(metadata, work, execution);
        if (BehaviorJson.has(metadata, 'cacheEvict') || BehaviorJson.has(metadata, 'cachePut')) {
          return _mutation(type, metadata, arguments, work, execution);
        }
        if (!BehaviorJson.has(metadata, 'cacheable')) return _resilient(type, method, metadata, work, execution);
        final policy = BehaviorJson.get(Behaviors.registration(type), 'cache');
        final key = await _authorize('cache:read', type, method, policy, metadata, arguments, execution);
        final cache = _cache(policy);
        final entry = await _adapter(execution, () => cache.get(key, execution));
        BehaviorJson.encode(entry);
        BehaviorJson.keys(entry, const ['hit', 'value', 'expiresAt']);
        if (BehaviorJson.boolean(BehaviorJson.get(entry, 'hit'))) {
          final expiry = BehaviorJson.integer(BehaviorJson.get(entry, 'expiresAt'));
          if (expiry > Behaviors.now + BehaviorJson.integer(BehaviorJson.get(policy, 'ttlMs'))) {
            throw BehaviorError('CACHE_EXPIRY');
          }
          if (expiry > Behaviors.now) {
            if (!BehaviorJson.has(entry, 'value')) throw BehaviorError('CACHE_VALUE');
            final value = BehaviorJson.get(entry, 'value');
            BehaviorJson.encode(value, BehaviorJson.integer(BehaviorJson.get(policy, 'maxBytes')));
            return value;
          }
        }
        final result = await _resilient(type, method, metadata, work, execution);
        execution.check();
        BehaviorJson.encode(result, BehaviorJson.integer(BehaviorJson.get(policy, 'maxBytes')));
        await _adapter(
            execution,
            () => cache.set(key, result, policy, Behaviors.now + BehaviorJson.integer(BehaviorJson.get(policy, 'ttlMs')),
                execution));
        return result;
      });

  Future<Object?> Function() _declaredBody(Object? metadata, List<Object?> arguments) {
    final declared = _exchangeRequest(BehaviorJson.get(metadata, 'httpExchange'), arguments);
    return () async => declared;
  }

  static Object? _withFilledPath(Object? metadata, List<Object?> arguments) {
    final policy = BehaviorJson.get(metadata, 'httpExchange');
    if (!BehaviorJson.has(policy, 'pathVariables')) return metadata;
    var path = BehaviorJson.text(BehaviorJson.get(policy, 'path'));
    BehaviorJson.members(BehaviorJson.get(policy, 'pathVariables')).forEach((name, index) {
      final at = BehaviorJson.integer(index);
      final value = at >= 0 && at < arguments.length ? arguments[at] : null;
      final text = value is String || value is num || value is bool ? '$value' : null;
      final allowed = text != null && RegExp(r'^[A-Za-z0-9._~-]{1,256}$').hasMatch(text) && text != '.' && text != '..';
      if (!allowed) throw BehaviorError('HTTP_ARGUMENT');
      path = path.replaceAll('{$name}', text);
    });
    return <String, Object?>{
      ...BehaviorJson.members(metadata),
      'httpExchange': <String, Object?>{...BehaviorJson.members(policy), 'path': path},
    };
  }

  static Object? _exchangeRequest(Object? policy, List<Object?> arguments) {
    Object? pick(Object? index) {
      final at = BehaviorJson.integer(index);
      if (at < 0 || at >= arguments.length) throw BehaviorError('HTTP_ARGUMENT');
      return arguments[at];
    }

    final query = <String, Object?>{};
    final headers = <String, Object?>{};
    if (BehaviorJson.has(policy, 'query')) {
      BehaviorJson.members(BehaviorJson.get(policy, 'query')).forEach((name, index) => query[name] = pick(index));
    }
    if (BehaviorJson.has(policy, 'headers')) {
      BehaviorJson.members(BehaviorJson.get(policy, 'headers')).forEach((name, index) => headers[name] = pick(index));
    }
    return <String, Object?>{
      'query': query,
      'headers': headers,
      if (BehaviorJson.has(policy, 'body')) 'body': pick(BehaviorJson.get(policy, 'body')),
    };
  }

  List<Object?> _mutationArguments(Object? mutation, List<Object?> arguments) {
    final normalized = BehaviorJson.items(BehaviorJson.get(mutation, 'arguments')).map((item) {
      final index = BehaviorJson.integer(item);
      if (index < 0 || index >= arguments.length) throw BehaviorError('CACHE_ARGUMENT');
      return arguments[index];
    }).toList();
    final target = BehaviorJson.get(mutation, 'target');
    if (BehaviorJson.has(target, 'pageable')) {
      final page = BehaviorJson.get(target, 'pageable');
      final index = BehaviorJson.integer(BehaviorJson.get(page, 'argument'));
      if (index < 0 || index >= normalized.length) throw BehaviorError('PAGE_ARGUMENT');
      normalized[index] = BehaviorPage.normalize(normalized[index], page);
    }
    return normalized;
  }

  Future<Object?> _mutation(String type, Object? metadata, List<Object?> arguments, Future<Object?> Function() body,
      BehaviorExecution execution) async {
    final put = BehaviorJson.has(metadata, 'cachePut');
    final mutation = BehaviorJson.get(metadata, put ? 'cachePut' : 'cacheEvict');
    final target = BehaviorJson.get(mutation, 'target');
    final targetMethod = BehaviorJson.text(BehaviorJson.get(target, 'method'));
    final policy = BehaviorJson.get(Behaviors.registration(type), 'cache');
    final normalized = _mutationArguments(mutation, arguments);
    final key =
        await _authorize(put ? 'cache:put' : 'cache:evict', type, targetMethod, policy, target, normalized, execution);
    final cache = _cache(policy);
    final result = await _resilient(type, targetMethod, metadata, body, execution);
    execution.check();
    BehaviorJson.encode(result, put ? BehaviorJson.integer(BehaviorJson.get(policy, 'maxBytes')) : BehaviorJson.limit);
    if (put) {
      await _adapter(
          execution,
          () => cache.set(key, result, policy, Behaviors.now + BehaviorJson.integer(BehaviorJson.get(policy, 'ttlMs')),
              execution));
    } else {
      await _adapter(execution, () => cache.delete(key, execution));
    }
    return result;
  }

  static bool _retryable(Object error) => error is! BehaviorError || error.code == 'ADAPTER_FAILED';

  Future<Object?> _transacted(Object? metadata, Future<Object?> Function() body, BehaviorExecution execution) async {
    if (!BehaviorJson.has(metadata, 'transactional')) return _attempt(metadata, body, execution);
    final name = BehaviorJson.text(BehaviorJson.get(BehaviorJson.get(metadata, 'transactional'), 'adapter'));
    if (BehaviorOpenTransactions.of(name) != null) return _attempt(metadata, body, execution);
    final adapter = _entities(name);
    Future<Object?> control(String action, [String? identifier]) => _adapter(
        execution,
        () => adapter.execute(
            {'operation': 'transaction', 'action': action, if (identifier != null) 'transaction': identifier}, execution));
    final begun = await control('begin');
    final identifier = BehaviorJson.get(begun, 'transaction');
    if (identifier is! String || identifier.isEmpty || behavior_convert.utf8.encode(identifier).length > 128) {
      throw BehaviorError('ENTITY_TRANSACTION');
    }
    final Object? value;
    try {
      value = await BehaviorOpenTransactions.within(name, identifier, () => _attempt(metadata, body, execution));
    } catch (_) {
      try {
        await control('rollback', identifier);
      } catch (_) {}
      rethrow;
    }
    await control('commit', identifier);
    return value;
  }

  Future<Object?> _resilient(
      String type, String method, Object? metadata, Future<Object?> Function() body, BehaviorExecution execution) async {
    if (!BehaviorJson.has(metadata, 'concurrencyLimit')) return _transacted(metadata, body, execution);
    final policy = BehaviorJson.get(metadata, 'concurrencyLimit');
    final key = '$type.$method';
    final limit = BehaviorJson.integer(BehaviorJson.get(policy, 'limit'));
    final blocking = BehaviorJson.text(BehaviorJson.get(policy, 'policy')) == 'block';
    while ((_slots[key] ?? 0) >= limit) {
      if (!blocking) throw BehaviorError('OPERATION_LIMIT');
      execution.check();
      await Future<void>.delayed(const Duration(milliseconds: 2));
      execution.check();
    }
    _slots[key] = (_slots[key] ?? 0) + 1;
    try {
      return await _transacted(metadata, body, execution);
    } finally {
      final taken = (_slots[key] ?? 1) - 1;
      if (taken <= 0) {
        _slots.remove(key);
      } else {
        _slots[key] = taken;
      }
    }
  }

  Future<Object?> _attempt(Object? metadata, Future<Object?> Function() body, BehaviorExecution execution) async {
    if (!BehaviorJson.has(metadata, 'retryable')) return _body(metadata, body, execution);
    final policy = BehaviorJson.get(metadata, 'retryable');
    final maxRetries = BehaviorJson.integer(BehaviorJson.get(policy, 'maxRetries'));
    final random = behavior_math.Random();
    for (var attempt = 0;; attempt++) {
      try {
        return await _body(metadata, body, execution);
      } catch (error) {
        if (attempt >= maxRetries || !_retryable(error)) rethrow;
        execution.check();
        final delay = behavior_math.min(
            BehaviorJson.integer(BehaviorJson.get(policy, 'delayMs')) *
                behavior_math.pow((BehaviorJson.get(policy, 'multiplier') as num).toDouble(), attempt),
            BehaviorJson.integer(BehaviorJson.get(policy, 'maxDelayMs')).toDouble());
        final wait = delay + random.nextDouble() * BehaviorJson.integer(BehaviorJson.get(policy, 'jitterMs'));
        if (wait > 0) await Future<void>.delayed(Duration(milliseconds: wait.round()));
        execution.check();
      }
    }
  }

  Map<String, Object?> _request(Object? descriptor, String method, String path, BehaviorExecution execution) => {
        ...BehaviorJson.members(descriptor),
        'query': BehaviorJson.get(descriptor, 'query') ?? const <String, Object?>{},
        'headers': BehaviorJson.get(descriptor, 'headers') ?? const <String, Object?>{},
        'method': method,
        'path': path,
        'redirect': 'error',
        'credentials': 'omit',
        'deadline': execution.deadline,
        'maxResponseBytes': BehaviorJson.limit,
      };

  Future<Object?> _body(Object? metadata, Future<Object?> Function() body, BehaviorExecution execution) async {
    execution.check();
    final result = await body();
    execution.check();
    BehaviorJson.encode(result);
    if (!BehaviorJson.has(metadata, 'httpExchange')) return result;
    BehaviorJson.keys(result, const ['query', 'headers', 'body']);
    _validateHttp(result);
    final policy = BehaviorJson.get(metadata, 'httpExchange');
    final method = BehaviorJson.text(BehaviorJson.get(policy, 'method'));
    if ((method == 'GET' || method == 'HEAD') && BehaviorJson.has(result, 'body')) throw BehaviorError('HTTP_BODY');
    final client = _adapters.http[BehaviorJson.text(BehaviorJson.get(policy, 'client'))] ?? (throw BehaviorError('HTTP_MISSING'));
    final request = _request(result, method, BehaviorJson.text(BehaviorJson.get(policy, 'path')), execution);
    return _adapter(execution, () => client.exchange(request, execution));
  }

  static final RegExp _token = RegExp(r"^[!#$%&'*+.^_`|~0-9A-Za-z-]+$");

  static void _validateHttp(Object? value) {
    if (BehaviorJson.has(value, 'headers')) {
      for (final entry in BehaviorJson.members(BehaviorJson.get(value, 'headers')).entries) {
        final text = BehaviorJson.text(entry.value);
        if (entry.key.isEmpty ||
            entry.key.length > 128 ||
            !_token.hasMatch(entry.key) ||
            behavior_convert.utf8.encode(text).length > 8192 ||
            text.contains(RegExp('[\r\n\u0000]')) ||
            const ['host', 'connection', 'content-length', 'transfer-encoding', 'cookie'].contains(entry.key.toLowerCase())) {
          throw BehaviorError('HTTP_HEADER');
        }
      }
    }
    if (BehaviorJson.has(value, 'query')) {
      for (final entry in BehaviorJson.members(BehaviorJson.get(value, 'query')).entries) {
        if (entry.key.length > 256) throw BehaviorError('HTTP_QUERY');
        BehaviorJson.encode(entry.value, 8192);
        if (entry.value == null || entry.value is Map || entry.value is List) throw BehaviorError('HTTP_QUERY');
      }
    }
  }

  Future<Object?> evict(String type, String method, List<Object?> arguments) => _operation((execution) async {
        final metadata = _method(type, method);
        if (!BehaviorJson.has(metadata, 'cacheable')) throw BehaviorError('CACHE_METHOD');
        final normalized = List<Object?>.of(arguments);
        if (BehaviorJson.has(metadata, 'pageable')) {
          final page = BehaviorJson.get(metadata, 'pageable');
          final index = BehaviorJson.integer(BehaviorJson.get(page, 'argument'));
          if (index < 0 || index >= normalized.length) throw BehaviorError('PAGE_ARGUMENT');
          normalized[index] = BehaviorPage.normalize(normalized[index], page);
        }
        final policy = BehaviorJson.get(Behaviors.registration(type), 'cache');
        final key = await _authorize('cache:evict', type, method, policy, metadata, normalized, execution);
        final cache = _cache(policy);
        await _adapter(execution, () => cache.delete(key, execution));
        return null;
      });

  BehaviorRestClient restClient(String name) {
    if (name.isEmpty || name.length > 128) throw BehaviorError('HTTP_MISSING');
    return BehaviorRestClient._(this, name);
  }

  Future<Object?> _restExchange(String name, String method, String path, Object? descriptor) =>
      _operation((execution) async {
        BehaviorJson.keys(descriptor, const ['query', 'headers', 'body']);
        _validateHttp(descriptor);
        if ((method == 'GET' || method == 'HEAD') && BehaviorJson.has(descriptor, 'body')) throw BehaviorError('HTTP_BODY');
        final client = _adapters.http[name] ?? (throw BehaviorError('HTTP_MISSING'));
        final request = _request(descriptor, method, path, execution);
        return _adapter(execution, () => client.exchange(request, execution));
      });

  static bool _safePath(String path) {
    if (!path.startsWith('/') || path.startsWith('//') || behavior_convert.utf8.encode(path).length > 2048) return false;
    if (path.runes.any((code) => code <= 32 || r'\?#%'.runes.contains(code))) return false;
    return !path.split('/').any((segment) => segment == '.' || segment == '..');
  }

  BehaviorRepository repository(Type entity) => BehaviorRepository._(this, Behaviors.entity(entity));

  static Object? mapValue(Object? plan, Object? value) {
    if (value is! Map) throw BehaviorError('JSON_OBJECT');
    final output = <String, Object?>{};
    for (final field in BehaviorJson.items(BehaviorJson.get(plan, 'fields'))) {
      final target = BehaviorJson.text(BehaviorJson.get(field, 'target'));
      final source = BehaviorJson.get(field, 'source');
      if (source is! String) {
        output[target] = BehaviorJson.get(field, 'constant');
        continue;
      }
      Object? current = value;
      for (final name in source.split('.')) {
        current = BehaviorJson.get(current, name);
      }
      output[target] = current;
    }
    BehaviorJson.encode(output);
    return output;
  }

  Future<Object?> query(Object? metadata, List<Object?> arguments) => _operation((execution) async {
        final command = _queryCommand(metadata, arguments);
        final adapterName = BehaviorJson.text(BehaviorJson.get(BehaviorJson.get(metadata, 'entity'), 'adapter'));
        final authorize = _adapters.authorizeEntity;
        final builtin = authorize == null && !_adapters.entities.containsKey(adapterName) && BehaviorStorage.entities(adapterName) != null;
        if (!builtin && authorize == null) throw BehaviorError('FORBIDDEN');
        if (authorize != null && !await _adapter(execution, () => authorize(command, execution))) throw BehaviorError('FORBIDDEN');
        final adapter = _entities(adapterName);
        final result = await _adapter(execution, () => adapter.execute(command, execution));
        BehaviorJson.encode(result);
        switch (BehaviorJson.text(BehaviorJson.get(metadata, 'operation'))) {
          case 'find' || 'findOne':
            final rows = BehaviorJson.items(result);
            final size = BehaviorJson.integer(
                BehaviorJson.get(BehaviorJson.get(BehaviorJson.get(command, 'query'), 'page'), 'size'));
            if (rows.length > size) throw BehaviorError('QUERY_RESULT');
            rows.forEach(BehaviorJson.members);
            if (BehaviorJson.get(metadata, 'operation') == 'findOne') return rows.isEmpty ? null : rows.first;
          case 'count' || 'delete':
            if (BehaviorJson.integer(result) < 0) throw BehaviorError('QUERY_RESULT');
          case 'exists':
            BehaviorJson.boolean(result);
          default:
            throw BehaviorError('QUERY_KIND');
        }
        return result;
      });

  static Object? _queryScalar(Object? value) {
    if (value is Map || value is List) throw BehaviorError('QUERY_BINDING');
    BehaviorJson.encode(value, 65536);
    return value;
  }

  static Object? _queryBinding(Object? condition, List<Object?> arguments) {
    final index = BehaviorJson.integer(BehaviorJson.get(condition, 'argument'));
    if (index < 0 || index >= arguments.length) throw BehaviorError('QUERY_ARGUMENT');
    final value = arguments[index];
    if (BehaviorJson.get(condition, 'operator') != 'in') return _queryScalar(value);
    final items = BehaviorJson.items(value);
    if (items.length > 100) throw BehaviorError('QUERY_BINDING');
    items.forEach(_queryScalar);
    BehaviorJson.encode(value, 65536);
    return value;
  }

  Object? _queryCommand(Object? metadata, List<Object?> arguments) {
    final fields = BehaviorJson.items(BehaviorJson.get(metadata, 'fields')).map(BehaviorJson.text).toSet();
    final conditions = <Object?>[];
    for (final condition in BehaviorJson.items(BehaviorJson.get(metadata, 'where'))) {
      final field = BehaviorJson.text(BehaviorJson.get(condition, 'field'));
      final operation = BehaviorJson.text(BehaviorJson.get(condition, 'operator'));
      if (!fields.contains(field) ||
          !const ['eq', 'ne', 'lt', 'lte', 'gt', 'gte', 'in', 'isNull', 'isNotNull'].contains(operation)) {
        throw BehaviorError('QUERY_BINDING');
      }
      conditions.add({
        'field': field,
        'operator': operation,
        if (operation != 'isNull' && operation != 'isNotNull') 'value': _queryBinding(condition, arguments),
      });
    }
    BehaviorJson.encode(conditions, 65536);
    final order = BehaviorJson.get(metadata, 'orderBy');
    for (final item in BehaviorJson.items(order)) {
      if (!fields.contains(BehaviorJson.text(BehaviorJson.get(item, 'field'))) ||
          !const ['asc', 'desc'].contains(BehaviorJson.text(BehaviorJson.get(item, 'direction')))) {
        throw BehaviorError('QUERY_ORDER');
      }
    }
    final kind = BehaviorJson.text(BehaviorJson.get(metadata, 'operation'));
    if (!const ['find', 'findOne', 'count', 'exists', 'delete'].contains(kind)) throw BehaviorError('QUERY_KIND');
    final single = kind == 'findOne';
    final query = <String, Object?>{'kind': single ? 'find' : kind, 'where': conditions, 'orderBy': order};
    if (single) {
      query['page'] = BehaviorPage.normalize(const <String, Object?>{'size': 1}, {
        'defaultSize': 1,
        'maxSize': 1,
        'sort': BehaviorJson.get(metadata, 'fields'),
      });
    }
    if (kind == 'find') {
      Object? input = const <String, Object?>{};
      final argument = BehaviorJson.get(metadata, 'pageArgument');
      if (argument != null) {
        final index = BehaviorJson.integer(argument);
        if (index < 0 || index >= arguments.length) throw BehaviorError('QUERY_ARGUMENT');
        input = arguments[index];
      }
      final page = BehaviorPage.normalize(input, {
        'defaultSize': BehaviorJson.get(metadata, 'defaultSize'),
        'maxSize': BehaviorJson.get(metadata, 'maxSize'),
        'sort': BehaviorJson.get(metadata, 'fields'),
      });
      if (BehaviorJson.items(BehaviorJson.get(page, 'sort')).isNotEmpty) throw BehaviorError('QUERY_ORDER');
      query['page'] = page;
    }
    final entity = BehaviorJson.get(metadata, 'entity');
    final joined = BehaviorOpenTransactions.of(BehaviorJson.text(BehaviorJson.get(entity, 'adapter')));
    return <String, Object?>{
      'operation': 'query',
      'entity': BehaviorJson.get(entity, 'name'),
      'table': BehaviorJson.get(entity, 'table'),
      'idField': BehaviorJson.get(entity, 'id'),
      'query': query,
      if (joined != null) 'transaction': joined,
    };
  }

  Future<Object?> _executeEntity(Object? mapping, String operation, Object? value) => _operation((execution) async {
        final adapter = _entities(BehaviorJson.text(BehaviorJson.get(mapping, 'adapter')));
        final command = <String, Object?>{...BehaviorJson.members(mapping)}..remove('adapter');
        command['operation'] = operation;
        command[operation == 'find'
            ? 'page'
            : operation == 'save'
                ? 'value'
                : 'id'] = value;
        final joined = BehaviorOpenTransactions.of(BehaviorJson.text(BehaviorJson.get(mapping, 'adapter')));
        if (joined != null) command['transaction'] = joined;
        return _adapter(execution, () => adapter.execute(command, execution));
      });
}

final class BehaviorRestClient {
  BehaviorRestClient._(this._context, this._name);
  final BehaviorContext _context;
  final String _name;
  BehaviorRestCall get(String path) => BehaviorRestCall._(_context, _name, 'GET', path);
  BehaviorRestCall post(String path) => BehaviorRestCall._(_context, _name, 'POST', path);
  BehaviorRestCall put(String path) => BehaviorRestCall._(_context, _name, 'PUT', path);
  BehaviorRestCall patch(String path) => BehaviorRestCall._(_context, _name, 'PATCH', path);
  BehaviorRestCall delete(String path) => BehaviorRestCall._(_context, _name, 'DELETE', path);
  BehaviorRestCall head(String path) => BehaviorRestCall._(_context, _name, 'HEAD', path);
  BehaviorRestCall options(String path) => BehaviorRestCall._(_context, _name, 'OPTIONS', path);
}

final class BehaviorRestCall {
  BehaviorRestCall._(this._context, this._name, this._method, this._path) {
    if (!BehaviorContext._safePath(_path)) throw BehaviorError('HTTP_PATH');
  }
  final BehaviorContext _context;
  final String _name;
  final String _method;
  final String _path;
  final Map<String, Object?> _parameters = {};
  final Map<String, Object?> _fields = {};
  Object? _payload;
  bool _hasPayload = false;

  BehaviorRestCall query(Map<String, Object?> values) {
    _parameters.addAll(values);
    return this;
  }

  BehaviorRestCall header(String key, String value) {
    _fields[key] = value;
    return this;
  }

  BehaviorRestCall body(Object? value) {
    if (_method == 'GET' || _method == 'HEAD') throw BehaviorError('HTTP_BODY');
    _payload = value;
    _hasPayload = true;
    return this;
  }

  Future<Object?> retrieve() => _context._restExchange(_name, _method, _path,
      <String, Object?>{'query': _parameters, 'headers': _fields, if (_hasPayload) 'body': _payload});
}

final class BehaviorRepository {
  BehaviorRepository._(this._context, this._mapping);
  final BehaviorContext _context;
  final Object? _mapping;

  Future<Object?> find([Object? page = const <String, Object?>{}]) => _context._executeEntity(
      _mapping,
      'find',
      BehaviorPage.normalize(page, {
        'defaultSize': 20,
        'maxSize': 100,
        'sort': [BehaviorJson.get(_mapping, 'idField')],
      }));

  static void _identifier(Object? id) {
    if (id is String) {
      if (id.isEmpty || behavior_convert.utf8.encode(id).length > 256) throw BehaviorError('ENTITY_ID');
    } else {
      BehaviorJson.integer(id);
    }
  }

  Future<Object?> save(Object? value) {
    _identifier(BehaviorJson.get(value, BehaviorJson.text(BehaviorJson.get(_mapping, 'idField'))));
    BehaviorJson.encode(value);
    return _context._executeEntity(_mapping, 'save', value);
  }

  Future<Object?> delete(Object? id) {
    _identifier(id);
    return _context._executeEntity(_mapping, 'delete', id);
  }
}

abstract final class BehaviorCalls {
  static final List<String> _keys = <String>[];

  static void record(String key) {
    if (!_keys.contains(key)) _keys.add(key);
  }

  static List<String> take() {
    final keys = List<String>.of(_keys);
    _keys.clear();
    return keys;
  }
}
