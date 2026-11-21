import 'package:shelf/shelf.dart';
import 'models/Identity.dart';

@IdentityProvider
class AppIdentityProvider {
  static const _brokenToken = 'broken-token';

  static const _sessions = {
    'ada-token': Identity('ada', ['Admin']),
    'bob-token': Identity('bob', []),
  };

  Identity? identify(Request request) {
    final token = '${request.headers['authorization']}';
    _refuseBroken(token);
    return _find(token);
  }

  void _refuseBroken(String token) {
    if (token == _brokenToken) {
      throw Exception('the session store is down');
    }
  }

  Identity? _find(String token) {
    return _sessions[token];
  }
}
