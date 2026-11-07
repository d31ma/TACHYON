
class BehaviorAssertionFailure implements Exception {
  BehaviorAssertionFailure(this.message);

  final String message;

  @override
  String toString() => message;
}

class Assert {
  static const nothingThrown = 'Expected an error to be thrown, but nothing was thrown.';
  static const messageSeparator = ' ==> ';

  static void equals(Object? expected, Object? actual, [String? message]) {
    _check(_same(expected, actual), 'expected: <$expected> but was: <$actual>', message);
  }

  static void isTrue(bool condition, [String? message]) {
    _check(condition, 'expected: <true> but was: <$condition>', message);
  }

  static Future<Object> throws(Object? Function() action, [String? message]) async {
    try {
      await action();
    } catch (error) {
      return error;
    }
    throw BehaviorAssertionFailure(_text(nothingThrown, message));
  }

  static void _check(bool passed, String failure, String? message) {
    if (!passed) {
      throw BehaviorAssertionFailure(_text(failure, message));
    }
  }

  static String _text(String failure, String? message) {
    if (message == null) {
      return failure;
    }
    return '$message$messageSeparator$failure';
  }

  static bool _same(Object? expected, Object? actual) {
    if (expected is List && actual is List) {
      return _sameList(expected, actual);
    }
    if (expected is Map && actual is Map) {
      return _sameMap(expected, actual);
    }
    if (expected is Set && actual is Set) {
      return expected.length == actual.length && expected.containsAll(actual);
    }
    return expected == actual;
  }

  static bool _sameList(List<Object?> expected, List<Object?> actual) {
    if (expected.length != actual.length) {
      return false;
    }
    for (var index = 0; index < expected.length; index++) {
      if (!_same(expected[index], actual[index])) {
        return false;
      }
    }
    return true;
  }

  static bool _sameMap(Map<Object?, Object?> expected, Map<Object?, Object?> actual) {
    if (expected.length != actual.length) {
      return false;
    }
    for (final key in expected.keys) {
      if (!actual.containsKey(key) || !_same(expected[key], actual[key])) {
        return false;
      }
    }
    return true;
  }
}
