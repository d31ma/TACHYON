import 'package:shelf/shelf.dart';
import 'models/Identity.dart';

@IdentityProvider
class AppIdentityProvider {
  static const _sessions = {
    'ada-token': Identity('ada', ['Admin']),
    'bob-token': Identity('bob', []),
  };

  Identity? identify(Request request) {
    return _find('${request.headers['authorization']}');
  }

  Identity? _find(String token) {
    return _sessions[token];
  }
}
