import 'dart:io';

class Endpoint {
  @Setting("${yon.active.profile:development}")
  String profile = '';
}
