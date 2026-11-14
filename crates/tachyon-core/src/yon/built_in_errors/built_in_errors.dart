
final class BrokenRule {
  const BrokenRule(this.field, this.rule);

  final String field;
  final String rule;

  Map<String, Object?> toJson() => {'field': field, 'rule': rule};
}

abstract class YonBuiltInError implements Exception {
  const YonBuiltInError(this.status, this.reference);

  final int status;
  final String reference;

  @override
  String toString() => '$runtimeType';
}

final class NotSignedIn extends YonBuiltInError {
  const NotSignedIn([String reference = '']) : super(401, reference);
}

final class RoleMissing extends YonBuiltInError {
  const RoleMissing([String reference = '']) : super(403, reference);
}

final class IdentityCheckFailed extends YonBuiltInError {
  const IdentityCheckFailed([String reference = '']) : super(500, reference);
}

final class RequestRejected extends YonBuiltInError {
  const RequestRejected([String reference = '', this.part = '', this.field = '', this.message = ''])
      : super(400, reference);

  final String part;
  final String field;
  final String message;
}

final class BodyTooLarge extends YonBuiltInError {
  const BodyTooLarge([String reference = '', this.limitBytes = 0]) : super(413, reference);

  final int limitBytes;
}

final class CheckRejected extends YonBuiltInError {
  const CheckRejected([String reference = '', this.violations = const [], this.truncated = false])
      : super(422, reference);

  final List<BrokenRule> violations;
  final bool truncated;
}

final class UnexpectedError extends YonBuiltInError {
  const UnexpectedError([String reference = '']) : super(500, reference);
}

Object __yon_built_in_error__(List<Object?> values) {
  final Object? fields = values.isEmpty ? null : values.first;
  if (fields is! Map) throw StateError('Unknown built-in error.');
  final reference = __yon_built_in_text__(fields, 'reference');
  switch (__yon_built_in_text__(fields, 'type')) {
    case 'NotSignedIn':
      return NotSignedIn(reference);
    case 'RoleMissing':
      return RoleMissing(reference);
    case 'IdentityCheckFailed':
      return IdentityCheckFailed(reference);
    case 'RequestRejected':
      return RequestRejected(reference, __yon_built_in_text__(fields, 'part'),
          __yon_built_in_text__(fields, 'field'), __yon_built_in_text__(fields, 'message'));
    case 'BodyTooLarge':
      return BodyTooLarge(reference, __yon_built_in_number__(fields['limitBytes']));
    case 'CheckRejected':
      return CheckRejected(reference, __yon_built_in_rules__(fields['violations']), fields['truncated'] == true);
    case 'UnexpectedError':
      return UnexpectedError(reference);
  }
  throw StateError('Unknown built-in error.');
}

String __yon_built_in_text__(Map<Object?, Object?> fields, String name) {
  final Object? value = fields[name];
  return value is String ? value : '';
}

int __yon_built_in_number__(Object? value) {
  return value is num ? value.toInt() : 0;
}

List<BrokenRule> __yon_built_in_rules__(Object? entries) {
  if (entries is! List) return const [];
  return [
    for (final entry in entries)
      if (entry is Map) BrokenRule(__yon_built_in_text__(entry, 'field'), __yon_built_in_text__(entry, 'rule')),
  ];
}
