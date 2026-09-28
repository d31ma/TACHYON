class Endpoint {
  @Setting("${yon.server.port}")
  port;
  @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
  redis;
}
