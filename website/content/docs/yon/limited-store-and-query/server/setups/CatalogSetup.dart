@Setup
class CatalogSetup {
  static const rows = [
    {'id': 1, 'status': 'active'},
    {'id': 2, 'status': 'retired'},
  ];

  static BehaviorAdapters configure(BehaviorExecution execution) {
    return BehaviorAdapters(
      entities: {'catalog': BehaviorEntityAdapter(execute: findRows)},
      authorizeEntity: (command, execution) async => (command as Map)['entity'] == 'ProductEntity',
    );
  }

  static Future<Object?> findRows(Object? command, BehaviorExecution execution) async {
    final query = (command as Map)['query'] as Map;
    final condition = (query['where'] as List).first as Map;
    final keepUnequalRows = condition['operator'] == 'ne';
    final matchingRows = rows.where((row) {
      final isEqual = row[condition['field']] == condition['value'];
      if (keepUnequalRows) return !isEqual;
      return isEqual;
    }).toList();
    if (query['kind'] == 'count') return matchingRows.length;
    return matchingRows;
  }
}
