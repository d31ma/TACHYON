import 'package:shelf/shelf.dart';

class Middleware {
  Response? before(Request request) {
    final token = request.headers['authorization'];
    if (request.requestedUri.path.startsWith('/private') && token != 'let-me-in') {
      return Response(401, body: '{"error":"Unauthorized"}', headers: {'content-type': 'application/json'});
    }
    return null;
  }

  Response after(Request request) => Response(204, headers: {'x-served-by': 'tachyon'});
}
