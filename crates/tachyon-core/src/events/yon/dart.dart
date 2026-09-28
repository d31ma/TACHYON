final class ApplicationEventPublisher {
  T publishEvent<T>(T value, [bool retain = false]) {
    final plain = value == null || value is Map || value is Iterable || value is String || value is num || value is bool || value is Enum;
    if (plain) {
      yon_io.stderr.writeln('Yon local event publication failed.');
      return value;
    }
    __YonEvents.publish('${value.runtimeType}', __yon_json(value), retain);
    return value;
  }
}

final class __YonSubscription {
  __YonSubscription(this.name, this.callback, this.order, this.condition, this.models, this.returns, this.sequence);
  final String name;
  final Object? Function(Object?) callback;
  final int order;
  final Object? condition;
  final Map? models;
  final String? returns;
  final int sequence;
}

final class __YonEvents {
  __YonEvents([List<String> topics = const []]) : _topics = topics.toSet() {
    if (_active != null) throw StateError('Nested local event invocation');
    _active = this;
  }

  factory __YonEvents.delivery(String body, List<String> topics) {
    final events = __YonEvents(topics);
    final delivery = yon_convert.jsonDecode(body) as Map;
    events._only = {for (final key in delivery['listeners'] as List) '$key'};
    events._delivered = ('${delivery['topic']}', yon_convert.jsonEncode(delivery['payload']));
    return events;
  }

  static __YonEvents? _active;
  final List<(String, String)> _queue = [];
  final Map<String, String> _retained = {};
  final List<__YonSubscription> _listeners = [];
  final List<void Function()> destroyers = [];
  final List<(String, String)> _outbound = [];
  final Set<String> _topics;
  Set<String>? _only;
  (String, String)? _delivered;
  bool _closed = false;
  int _deliveries = 0;

  static T publish<T>(String name, T value, bool retain) {
    try {
      final scope = _active;
      if (scope == null || scope._closed) throw StateError('Local event owner is disposed');
      final json = yon_convert.jsonEncode(value);
      if (yon_convert.utf8.encode(json).length > 65536 || scope._queue.length >= 128) {
        throw StateError('Local event capacity exceeded');
      }
      if (retain) scope._retained[name] = json;
      scope._queue.add((name, json));
      if (scope._topics.contains(name)) scope._outbound.add((name, json));
    } catch (_) {
      yon_io.stderr.writeln('Yon local event publication failed.');
    }
    return value;
  }

  void receiveDelivery() {
    final delivered = _delivered;
    if (delivered != null) _queue.add(delivered);
  }

  void listen(String options, Object? Function(Object?) callback, bool replay, String key) {
    final only = _only;
    if (only != null && !only.contains(key)) return;
    final compiled = yon_convert.jsonDecode(options) as Map;
    final order = (compiled['order'] as num?)?.toInt() ?? 0;
    for (final name in compiled['events'] as List) {
      if (_listeners.length >= 1024) throw StateError('Local listener capacity exceeded');
      _listeners.add(__YonSubscription('$name', callback, order, compiled['condition'], compiled['models'] as Map?, compiled['returns'] as String?, _listeners.length));
      final retained = _retained['$name'];
      if (replay && retained != null) _queue.add(('$name', retained));
    }
    _listeners.sort((left, right) =>
        left.order != right.order ? left.order.compareTo(right.order) : left.sequence.compareTo(right.sequence));
  }

  void _republish(Object? result, String? returns) {
    if (returns == null) return;
    final items = result is List ? result.take(32) : [result];
    for (final item in items) {
      if (item != null) publish(returns, __yon_json(item), false);
    }
  }

  static bool _fits(Map models, String shape, Object? value) {
    if (shape.startsWith('?')) return value == null || _fits(models, shape.substring(1), value);
    if (shape.startsWith('[]')) return value is List && value.every((item) => _fits(models, shape.substring(2), item));
    switch (shape) {
      case 'any':
        return true;
      case 'string':
        return value is String;
      case 'integer':
        return value is int;
      case 'number':
        return value is num;
      case 'boolean':
        return value is bool;
    }
    final model = models[shape];
    if (model is! Map || value is! Map) return false;
    final fields = model['fields'] as Map;
    if (model['open'] != true && value.keys.any((key) => !fields.containsKey(key))) return false;
    for (final entry in fields.entries) {
      final field = '${entry.value}';
      final required = field.startsWith('!');
      if (!value.containsKey(entry.key)) {
        if (required) return false;
        continue;
      }
      if (!_fits(models, required ? field.substring(1) : field, value[entry.key])) return false;
    }
    return true;
  }

  Future<void> drain() async {
    while (_queue.isNotEmpty) {
      final (name, json) = _queue.removeAt(0);
      if (++_deliveries > 4096) {
        _queue.clear();
        yon_io.stderr.writeln('Yon local event delivery bound exceeded.');
        return;
      }
      for (final listener in List.of(_listeners)) {
        if (listener.name != name) continue;
        final value = yon_convert.jsonDecode(json);
        final models = listener.models;
        if (models != null && !_fits(models, name, value)) {
          yon_io.stderr.writeln('Yon local event payload does not fit the listener model.');
          continue;
        }
        if (listener.condition != null && !_matches(listener.condition, value, name)) continue;
        try {
          var result = listener.callback(value);
          if (result is Future) result = await result;
          _republish(result, listener.returns);
        } catch (_) {
          yon_io.stderr.writeln('Yon local event listener failed.');
        }
      }
    }
    await _sendTopics();
  }

  Future<void> _sendTopics() async {
    for (final (name, json) in List.of(_outbound)) {
      try {
        await YonManagedResources.call({'operation': 'topic', 'topic': name, 'payload_json': json});
      } catch (_) {
        yon_io.stderr.writeln('Yon topic publication failed.');
      }
    }
    _outbound.clear();
  }

  void close() {
    if (_closed) return;
    _closed = true;
    _queue.clear();
    _listeners.clear();
    for (final destroy in destroyers.reversed) {
      try {
        destroy();
      } catch (_) {
        yon_io.stderr.writeln('Yon local lifecycle cleanup failed.');
      }
    }
    _retained.clear();
    _active = null;
  }

  static bool _matches(Object? matcher, Object? value, String event) {
    try {
      return _test(matcher as Map, value, event);
    } catch (_) {
      return false;
    }
  }

  static Object? _operand(Map node, Object? value, String event) {
    switch (node['k']) {
      case 'lit':
        return node['v'];
      case 'event':
        return event;
    }
    var current = value;
    for (final segment in (node['path'] as List?) ?? const []) {
      if (current is Map) {
        current = current['$segment'];
      } else if (current is List && segment is num && segment.toInt() >= 0 && segment.toInt() < current.length) {
        current = current[segment.toInt()];
      } else {
        return null;
      }
    }
    return current;
  }

  static bool _truthy(Object? value) => switch (value) {
        null => false,
        bool flag => flag,
        num number => number != 0,
        String text => text.isNotEmpty,
        _ => true,
      };

  static bool _test(Map node, Object? value, String event) => switch (node['k']) {
        'and' => _test(node['l'] as Map, value, event) && _test(node['r'] as Map, value, event),
        'or' => _test(node['l'] as Map, value, event) || _test(node['r'] as Map, value, event),
        'not' => !_test(node['e'] as Map, value, event),
        'truthy' => _truthy(_operand(node['e'] as Map, value, event)),
        'cmp' => _compare('${node['op']}', _operand(node['l'] as Map, value, event), _operand(node['r'] as Map, value, event)),
        _ => false,
      };

  static bool _compare(String operation, Object? left, Object? right) {
    final numbers = left is num && right is num;
    final strings = left is String && right is String;
    if (operation == 'eq' || operation == 'ne') {
      final equal = numbers
          ? (left as num).toDouble() == (right as num).toDouble()
          : strings || left is bool
              ? left == right
              : left == null && right == null;
      return (operation == 'eq') == equal;
    }
    final int order;
    if (numbers) {
      order = (left as num).toDouble().compareTo((right as num).toDouble());
    } else if (strings) {
      order = (left as String).compareTo(right as String);
    } else {
      return false;
    }
    return switch (operation) {
      'lt' => order < 0,
      'le' => order <= 0,
      'gt' => order > 0,
      _ => order >= 0,
    };
  }
}
