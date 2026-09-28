import 'dart:convert';
import 'dart:io';

class Endpoint {
  @Setting("${yon.server.port}")
  int port = 0;
  @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
  String redis = '';
}
