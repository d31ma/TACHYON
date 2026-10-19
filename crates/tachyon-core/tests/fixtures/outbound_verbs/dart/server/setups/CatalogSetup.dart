@Setup
class CatalogSetup {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return BehaviorAdapters(
      http: {'catalog': BehaviorHttpAdapter(exchange: echo)},
    );
  }

  static Future<Object?> echo(Object? request, BehaviorExecution execution) async {
    final sent = request as Map<String, Object?>;
    return {for (final key in const ['method', 'path', 'query', 'headers', 'body']) key: sent[key]};
  }
}
