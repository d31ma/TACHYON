class Endpoint {
  @Setting("${yon.server.port}")
  final int port;
  @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
  final String redis;

  const Endpoint({this.port = 0, this.redis = ''});
}
