import 'dart:math';

@Setup
class PageProgramSetup {
  static const _identityBytes = 16;
  static const _byteValues = 256;
  static const _hexadecimal = 16;
  static const _hexDigits = 2;

  static BehaviorAdapters configure(BehaviorExecution execution) {
    final identity = _newIdentity();
    var count = 0;
    return BehaviorAdapters(
      http: {
        'pageprogram': BehaviorHttpAdapter(exchange: (request, running) async {
          final method = BehaviorJson.get(request, 'method');
          final path = BehaviorJson.text(BehaviorJson.get(request, 'path'));
          if (method == 'POST' && path.endsWith('/increment')) count += 1;
          return {'identity': identity, 'count': count};
        }),
      },
    );
  }

  static String _newIdentity() {
    final random = Random.secure();
    return List.generate(_identityBytes, (_) => random.nextInt(_byteValues).toRadixString(_hexadecimal).padLeft(_hexDigits, '0')).join();
  }
}
