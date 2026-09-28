@Helper
final class LedgerAdapterHelper {
  static const transactionKey = 'transaction';
  static final List<String> commands = [];
  static int _sequence = 0;

  static Future<Object?> execute(Object? command, BehaviorExecution execution) async {
    final fields = command as Map;
    final identifier = fields[transactionKey];
    if (fields['operation'] == transactionKey) {
      var entry = '${fields['action']}';
      if (identifier != null) entry = '$entry:$identifier';
      commands.add(entry);
      if (fields['action'] != 'begin') return {'ok': true};
      _sequence += 1;
      return {transactionKey: 't$_sequence'};
    }
    final query = fields['query'] as Map;
    final transaction = identifier ?? 'none';
    commands.add('${query['kind']}@$transaction');
    if (query['kind'] == 'count') return 1;
    return [
      {'id': 1, 'status': 'open'},
    ];
  }
}
